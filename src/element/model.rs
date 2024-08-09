use crate::Result;
use crate::{osm::overpass::OverpassElement, Error};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::time::sleep;
use tracing::{debug, info};

pub struct ElementRepo {
    pool: Arc<Pool>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Element {
    pub id: i64,
    pub overpass_data: OverpassElement,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl ElementRepo {
    pub fn new(pool: &Arc<Pool>) -> Self {
        Self { pool: pool.clone() }
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            pool: Arc::new(crate::test::mock_db().1),
        }
    }

    #[cfg(test)]
    pub async fn insert(&self, overpass_data: &OverpassElement) -> Result<Element> {
        let overpass_data = overpass_data.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::insert(&overpass_data, conn))
            .await?
    }

    pub async fn select_all(&self, limit: Option<i64>) -> Result<Vec<Element>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::select_all(limit, conn))
            .await?
    }

    pub async fn select_updated_since(
        &self,
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
    ) -> Result<Vec<Element>> {
        let updated_since = updated_since.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::select_updated_since(&updated_since, limit, conn))
            .await?
    }

    pub async fn select_by_id(&self, id: i64) -> Result<Option<Element>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::select_by_id(id, conn))
            .await?
    }

    pub async fn select_by_osm_type_and_id(
        &self,
        r#type: &str,
        id: i64,
    ) -> Result<Option<Element>> {
        let r#type = r#type.to_string();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::select_by_osm_type_and_id(&r#type, id, conn))
            .await?
    }

    pub async fn patch_tags(&self, id: i64, tags: &Map<String, Value>) -> Result<Element> {
        let tags = tags.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::_patch_tags(id, &tags, conn))
            .await?
    }

    pub async fn set_tag(&self, id: i64, name: &str, value: &Value) -> Result<Element> {
        sleep(Duration::from_millis(10)).await;
        let name = name.to_string();
        let value = value.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::_set_tag(id, &name, &value, conn))
            .await?
    }

    pub async fn remove_tag(&self, id: i64, name: &str) -> Result<Element> {
        sleep(Duration::from_millis(10)).await;
        let name = name.to_string();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::_remove_tag(id, &name, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn set_updated_at(&self, id: i64, updated_at: &OffsetDateTime) -> Result<Element> {
        let updated_at = updated_at.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Element::_set_updated_at(id, &updated_at, conn))
            .await?
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
    pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_OVERPASS_DATA}) 
                VALUES (json(:overpass_data))
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! { ":overpass_data": serde_json::to_string(overpass_data)?},
        )?;
        Ok(Element::select_by_id(conn.last_insert_rowid(), &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Element>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS} 
                FROM {TABLE} 
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID} 
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;

        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded all elements ({}) in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Element>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
        );
        debug!(query);
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX)
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Element>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_by_osm_type_and_id(
        r#type: &str,
        id: i64,
        conn: &Connection,
    ) -> Result<Option<Element>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE json_extract({COL_OVERPASS_DATA}, '$.type') = :type
                AND json_extract({COL_OVERPASS_DATA}, '$.id') = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(
                &query,
                named_params! {
                    ":type": r#type,
                    ":id": id,
                },
                mapper(),
            )
            .optional()?)
    }

    #[cfg(test)]
    pub fn patch_tags(
        &self,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE} SET {COL_TAGS} = json_patch({COL_TAGS}, :tags) WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": self.id,
                ":tags": &serde_json::to_string(tags)?,
            },
        )?;
        Ok(Element::select_by_id(self.id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn _patch_tags(
        id: i64,
        tags: &Map<String, Value>,
        conn: &Connection,
    ) -> crate::Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE} SET {COL_TAGS} = json_patch({COL_TAGS}, :tags) WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":tags": &serde_json::to_string(tags)?,
            },
        )?;
        Ok(Element::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn set_overpass_data(
        &self,
        overpass_data: &OverpassElement,
        conn: &Connection,
    ) -> Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_OVERPASS_DATA} = json(:overpass_data)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": self.id,
                ":overpass_data": serde_json::to_string(overpass_data)?,
            },
        )?;
        Ok(Element::select_by_id(self.id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn set_tag(&self, name: &str, value: &Value, conn: &Connection) -> Result<Element> {
        Element::_set_tag(self.id, name, value, conn)
    }

    pub fn _set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<Element> {
        let mut patch_set = Map::new();
        patch_set.insert(name.into(), value.clone());
        Element::_patch_tags(id, &patch_set, conn)
    }

    pub fn remove_tag(&self, name: &str, conn: &Connection) -> Result<Element> {
        Element::_remove_tag(self.id, name, conn)
    }

    pub fn _remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_remove(tags, :name)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":name": format!("$.{name}"),
            },
        )?;
        info!("Removed {name} tag from element {id}");
        Ok(Element::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
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
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339).unwrap(),
            },
        )?;
        Ok(Element::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn set_deleted_at(
        &self,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Element> {
        match deleted_at {
            Some(deleted_at) => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :deleted_at
                        WHERE {COL_ROWID} = :id
                    "#
                );
                debug!(query);
                conn.execute(
                    &query,
                    named_params! {
                        ":id": self.id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ROWID} = :id
                    "#
                );
                debug!(query);
                conn.execute(&query, named_params! { ":id": self.id })?;
            }
        };
        Ok(Element::select_by_id(self.id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let overpass_data: String = row.get(1)?;
        let overpass_data: OverpassElement = serde_json::from_str(&overpass_data).unwrap();
        let tags: String = row.get(2)?;
        Ok(Element {
            id: row.get(0)?,
            overpass_data: overpass_data,
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::json;
    use time::{macros::datetime, OffsetDateTime};

    use crate::{osm::overpass::OverpassElement, test::mock_conn, Result};

    use super::Element;

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
            Element::select_updated_since(&datetime!(2023-10-01 00:00 UTC), None, &conn)?
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
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let element = element.patch_tags(&tags, &conn)?;
        assert_eq!(&tag_1_value_1, element.tag(tag_1_name));
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let element = element.patch_tags(&tags, &conn)?;
        assert_eq!(&tag_1_value_2, element.tag(tag_1_name));
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let element = element.patch_tags(&tags, &conn)?;
        assert!(element.tags.contains_key(tag_1_name));
        assert_eq!(&tag_2_value, element.tag(tag_2_name));
        Ok(())
    }

    #[test]
    fn set_overpass_data() -> Result<()> {
        let conn = mock_conn();
        let orig_data = OverpassElement::mock(1);
        let override_data = OverpassElement::mock(2);
        let element =
            Element::insert(&orig_data, &conn)?.set_overpass_data(&override_data, &conn)?;
        assert_eq!(override_data, element.overpass_data);
        Ok(())
    }

    #[test]
    fn set_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let tag_value = json!("bar");
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .set_tag(tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, element.tags[tag_name]);
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .set_tag(tag_name, &"bar".into(), &conn)?
            .remove_tag(tag_name, &conn)?;
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
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .set_deleted_at(Some(deleted_at), &conn)?;
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
