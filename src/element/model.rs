use crate::osm::overpass::OverpassElement;
use crate::{db, Result};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection};
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
const _ALL_COLUMNS: &str = "rowid, overpass_data, tags, created_at, updated_at, deleted_at";
const COL_ROWID: &str = "rowid";
const _COL_OVERPASS_DATA: &str = "overpass_data";
const COL_TAGS: &str = "tags";
const _COL_CREATED_AT: &str = "created_at";
#[cfg(test)]
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Element {
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
            .interact(move |conn| db::element::queries::set_tag(id, &name, &value, conn))
            .await?
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
        db::element::queries::select_by_id(id, conn)
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
        db::element::queries::select_by_id(id, conn)
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
        db::element::queries::select_by_id(id, conn)
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

#[cfg(test)]
mod test {
    use super::Element;
    use crate::{db, osm::overpass::OverpassElement, test::mock_conn, Result};
    use serde_json::{json, Map};
    use time::OffsetDateTime;

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value_1 = json!("tag_1_value_1");
        let tag_1_value_2 = json!("tag_1_value_2");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let element = db::element::queries::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_1, element.tag(tag_1_name));
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let element = db::element::queries::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_2, element.tag(tag_1_name));
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let element = db::element::queries::patch_tags(element.id, &tags, &conn)?;
        assert!(element.tags.contains_key(tag_1_name));
        assert_eq!(&tag_2_value, element.tag(tag_2_name));
        Ok(())
    }

    #[test]
    fn set_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let tag_value = json!("bar");
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let element = db::element::queries::set_tag(element.id, tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, element.tags[tag_name]);
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let element = db::element::queries::set_tag(element.id, tag_name, &"bar".into(), &conn)?;
        let element = Element::remove_tag(element.id, tag_name, &conn)?;
        assert!(!element.tags.contains_key(tag_name));
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = mock_conn();
        let updated_at = OffsetDateTime::now_utc();
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&updated_at, &conn)?;
        assert_eq!(
            updated_at,
            db::element::queries::select_by_id(element.id, &conn)?.updated_at
        );
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let deleted_at = OffsetDateTime::now_utc();
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let element = Element::set_deleted_at(element.id, Some(deleted_at), &conn)?;
        assert_eq!(
            deleted_at,
            db::element::queries::select_by_id(element.id, &conn)?
                .deleted_at
                .unwrap()
        );
        Ok(())
    }
}
