use crate::Result;
use crate::{osm::overpass::OverpassElement, Error};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::{Map, Value};
use std::hash::Hash;
use std::hash::Hasher;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, Serialize)]
pub struct Element {
    pub id: i64,
    pub overpass_data: OverpassElement,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Element {}

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

const TABLE: &str = "element";
const ALL_COLUMNS: &str = "rowid, overpass_data, tags, created_at, updated_at, deleted_at";
const COL_ROWID: &str = "rowid";
const COL_OVERPASS_DATA: &str = "overpass_data";
const COL_TAGS: &str = "tags";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Element {
    pub async fn insert_async(overpass_data: OverpassElement, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::insert(&overpass_data, conn))
            .await?
    }

    pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
        let sql = format!(
            r#"
                INSERT INTO {TABLE} ({COL_OVERPASS_DATA}) 
                VALUES (json(:overpass_data))
            "#
        );
        conn.execute(
            &sql,
            named_params! { ":overpass_data": serde_json::to_string(overpass_data)?},
        )?;
        Element::select_by_id(conn.last_insert_rowid(), conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub async fn select_all_async(limit: Option<i64>, pool: &Pool) -> Result<Vec<Element>> {
        pool.get()
            .await?
            .interact(move |conn| Element::select_all(limit, conn))
            .await?
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Element>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS} 
                FROM {TABLE} 
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID} 
                LIMIT :limit
            "#
        );
        let res = conn
            .prepare(&sql)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub async fn select_all_except_deleted_async(pool: &Pool) -> Result<Vec<Element>> {
        pool.get()
            .await?
            .interact(move |conn| Element::select_all_except_deleted(conn))
            .await?
    }

    pub fn select_all_except_deleted(conn: &Connection) -> Result<Vec<Element>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS} 
                FROM {TABLE}
                WHERE {COL_DELETED_AT} IS NULL
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
            "#
        );
        let res = conn
            .prepare(&sql)?
            .query_map({}, mapper())?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        include_deleted: bool,
        conn: &Connection,
    ) -> Result<Vec<Element>> {
        let sql = if include_deleted {
            format!(
                r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
            )
        } else {
            format!(
                r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE deleted_at IS NULL AND {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
            )
        };
        Ok(conn
            .prepare(&sql)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX)
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn select_by_search_query_async(
        search_query: impl Into<String>,
        pool: &Pool,
    ) -> Result<Vec<Self>> {
        let search_query = search_query.into();
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_search_query(&search_query, conn))
            .await?
    }

    pub fn select_by_search_query(
        search_query: impl Into<String>,
        conn: &Connection,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE LOWER(json_extract({COL_OVERPASS_DATA}, '$.tags.name')) LIKE '%' || UPPER(:query) || '%'
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID};
            "#
        );
        conn.prepare(&sql)?
            .query_map(named_params! { ":query": search_query.into() }, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn select_by_id_or_osm_id_async(
        id: impl Into<String>,
        pool: &Pool,
    ) -> Result<Option<Element>> {
        let id = id.into();
        pool.get()
            .await?
            .interact(|conn| Element::select_by_id_or_osm_id(id, conn))
            .await?
    }

    pub fn select_by_id_or_osm_id(
        id: impl Into<String>,
        conn: &Connection,
    ) -> Result<Option<Element>> {
        let id: String = id.into();
        let id = id.as_str();
        match id.parse::<i64>() {
            Ok(id) => Element::select_by_id(id, conn),
            Err(_) => {
                let parts: Vec<_> = id.split(':').collect();
                let osm_type = parts[0];
                let osm_id = parts[1].parse::<i64>().unwrap();
                Element::select_by_osm_type_and_id(osm_type, osm_id, conn)
            }
        }
    }

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Element> {
        pool.get()
            .await?
            .interact(move |conn| Element::select_by_id(id, conn))
            .await??
            .ok_or(Error::not_found())
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Element>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ROWID} = :id
            "#
        );
        Ok(conn
            .query_row(&sql, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_by_osm_type_and_id(
        r#type: &str,
        id: i64,
        conn: &Connection,
    ) -> Result<Option<Element>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE json_extract({COL_OVERPASS_DATA}, '$.type') = :type
                AND json_extract({COL_OVERPASS_DATA}, '$.id') = :id
            "#
        );
        Ok(conn
            .query_row(
                &sql,
                named_params! {
                    ":type": r#type,
                    ":id": id,
                },
                mapper(),
            )
            .optional()?)
    }

    pub fn patch_tags(
        id: i64,
        tags: &Map<String, Value>,
        conn: &Connection,
    ) -> crate::Result<Element> {
        let sql = format!(
            r#"
                UPDATE {TABLE} SET {COL_TAGS} = json_patch({COL_TAGS}, :tags) WHERE {COL_ROWID} = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":tags": &serde_json::to_string(tags)?,
            },
        )?;
        Element::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub async fn set_overpass_data_async(
        id: i64,
        overpass_data: OverpassElement,
        pool: &Pool,
    ) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_overpass_data(id, &overpass_data, conn))
            .await?
    }

    pub fn set_overpass_data(
        id: i64,
        overpass_data: &OverpassElement,
        conn: &Connection,
    ) -> Result<Element> {
        let sql = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_OVERPASS_DATA} = json(:overpass_data)
                WHERE {COL_ROWID} = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":overpass_data": serde_json::to_string(overpass_data)?,
            },
        )?;
        Element::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub async fn set_tag_async(
        id: i64,
        name: impl Into<String>,
        value: &Value,
        pool: &Pool,
    ) -> Result<Element> {
        let name = name.into();
        let value = value.clone();
        pool.get()
            .await?
            .interact(move |conn| Element::set_tag(id, &name, &value, conn))
            .await?
    }

    pub fn set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<Element> {
        let mut patch_set = Map::new();
        patch_set.insert(name.into(), value.clone());
        Element::patch_tags(id, &patch_set, conn)
    }

    pub async fn remove_tag_async(
        element_id: i64,
        tag_name: impl Into<String>,
        pool: &Pool,
    ) -> Result<Element> {
        let tag_name = tag_name.into();
        pool.get()
            .await?
            .interact(move |conn| Element::remove_tag(element_id, &tag_name, conn))
            .await?
    }

    pub fn remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Element> {
        let sql = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_remove(tags, :name)
                WHERE {COL_ROWID} = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":name": format!("$.{name}"),
            },
        )?;
        Element::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    #[cfg(test)]
    pub fn set_updated_at(
        &self,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Element> {
        Element::_set_updated_at(self.id, updated_at, conn)
    }

    #[cfg(test)]
    pub fn _set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Element> {
        let sql = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ROWID} = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339).unwrap(),
            },
        )?;
        Element::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub async fn set_deleted_at_async(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        pool: &Pool,
    ) -> Result<Element> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_deleted_at(id, deleted_at, conn))
            .await?
    }

    pub fn set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Element> {
        match deleted_at {
            Some(deleted_at) => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :deleted_at
                        WHERE {COL_ROWID} = :id
                    "#
                );
                conn.execute(
                    &sql,
                    named_params! {
                        ":id": id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ROWID} = :id
                    "#
                );
                conn.execute(&sql, named_params! { ":id": id })?;
            }
        };
        Element::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }

    pub fn name(&self) -> String {
        self.overpass_data.tag("name").into()
    }

    pub fn osm_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/{}/{}",
            self.overpass_data.r#type, self.overpass_data.id,
        )
    }

    pub fn lat(&self) -> f64 {
        self.overpass_data.coord().y
    }

    pub fn lon(&self) -> f64 {
        self.overpass_data.coord().x
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let overpass_data: String = row.get(1)?;
        let overpass_data: OverpassElement = serde_json::from_str(&overpass_data).unwrap();
        let tags: String = row.get(2)?;
        Ok(Element {
            id: row.get(0)?,
            overpass_data,
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::Element;
    use crate::{osm::overpass::OverpassElement, test::mock_conn, Result};
    use serde_json::{json, Map};
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let overpass_data = OverpassElement::mock(1);
        let element = Element::insert(&overpass_data, &conn)?;
        assert_eq!(overpass_data, element.overpass_data);
        let element = Element::select_by_id(1, &conn)?;
        assert!(element.is_some());
        assert_eq!(overpass_data, element.unwrap().overpass_data);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        assert_eq!(
            vec![
                Element::insert(&OverpassElement::mock(1), &conn)?,
                Element::insert(&OverpassElement::mock(2), &conn)?,
                Element::insert(&OverpassElement::mock(3), &conn)?
            ],
            Element::select_all(None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&datetime!(2023-10-01 00:00 UTC), &conn)?;
        let expected_element = Element::insert(&OverpassElement::mock(2), &conn)?
            .set_updated_at(&datetime!(2023-10-02 00:00 UTC), &conn)?;
        assert_eq!(
            vec![expected_element],
            Element::select_updated_since(&datetime!(2023-10-01 00:00 UTC), None, true, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(element, Element::select_by_id(element.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn select_by_osm_type_and_id() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(
            element,
            Element::select_by_osm_type_and_id(
                &element.overpass_data.r#type,
                element.overpass_data.id,
                &conn,
            )?
            .unwrap()
        );
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value_1 = json!("tag_1_value_1");
        let tag_1_value_2 = json!("tag_1_value_2");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let element = Element::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_1, element.tag(tag_1_name));
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let element = Element::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_2, element.tag(tag_1_name));
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let element = Element::patch_tags(element.id, &tags, &conn)?;
        assert!(element.tags.contains_key(tag_1_name));
        assert_eq!(&tag_2_value, element.tag(tag_2_name));
        Ok(())
    }

    #[test]
    fn set_overpass_data() -> Result<()> {
        let conn = mock_conn();
        let orig_data = OverpassElement::mock(1);
        let override_data = OverpassElement::mock(2);
        let element = Element::insert(&orig_data, &conn)?;
        let element = Element::set_overpass_data(element.id, &override_data, &conn)?;
        assert_eq!(override_data, element.overpass_data);
        Ok(())
    }

    #[test]
    fn set_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let tag_value = json!("bar");
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let element = Element::set_tag(element.id, tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, element.tags[tag_name]);
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let element = Element::set_tag(element.id, tag_name, &"bar".into(), &conn)?;
        let element = Element::remove_tag(element.id, tag_name, &conn)?;
        assert!(!element.tags.contains_key(tag_name));
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = mock_conn();
        let updated_at = OffsetDateTime::now_utc();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&updated_at, &conn)?;
        assert_eq!(
            updated_at,
            Element::select_by_id(element.id, &conn)?
                .unwrap()
                .updated_at
        );
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let deleted_at = OffsetDateTime::now_utc();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let element = Element::set_deleted_at(element.id, Some(deleted_at), &conn)?;
        assert_eq!(
            deleted_at,
            Element::select_by_id(element.id, &conn)?
                .unwrap()
                .deleted_at
                .unwrap()
        );
        Ok(())
    }
}
