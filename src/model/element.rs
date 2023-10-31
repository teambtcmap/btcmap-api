use std::collections::HashMap;

use rusqlite::named_params;
use rusqlite::OptionalExtension;

use rusqlite::Connection;
use rusqlite::Row;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::debug;

use crate::service::overpass::OverpassElement;
use crate::Error;
use crate::Result;

#[derive(PartialEq, Debug, Clone)]
pub struct Element {
    pub id: i64,
    pub overpass_data: OverpassElement,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "element";
const ALL_COLUMNS: &str = &format!("{COL_ROWID}, {COL_OVERPASS_DATA}, {COL_TAGS}, {COL_CREATED_AT}, {COL_UPDATED_AT}, {COL_DELETED_AT}");
const COL_ROWID: &str = "rowid";
const COL_OVERPASS_DATA: &str = "overpass_data";
const COL_TAGS: &str = "tags";
const COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Element {
    pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_OVERPASS_DATA}) 
                VALUES (:overpass_data)
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! { ":overpass_data": serde_json::to_string(overpass_data)?},
        )?;
        Ok(Element::select_by_id(conn.last_insert_rowid(), &conn)?
            .ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Element>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS} 
                FROM {TABLE} 
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID} 
                LIMIT :limit
            "#
        );
        debug!(query);
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
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

    // pub fn set_tags(&self, tags: &HashMap<String, Value>, conn: &Connection) -> Result<Element> {
    //     conn.execute(
    //         "UPDATE element SET tags = json(:tags) WHERE rowid = :id",
    //         named_params! {
    //             ":id": self.id,
    //             ":tags": serde_json::to_string(tags)?,
    //         },
    //     )?;
    //     Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    // }

    pub fn set_overpass_data(
        &self,
        overpass_data: &OverpassElement,
        conn: &Connection,
    ) -> Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_OVERPASS_DATA} = json(:overpass_json)
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
        Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn insert_tag(&self, name: &str, value: &str, conn: &Connection) -> Result<Element> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_set(tags, :name, :value)
                WHERE id = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": self.id,
                ":name": format!("$.{name}"),
                ":value": value,
            },
        )?;
        Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn delete_tag(&self, name: &str, conn: &Connection) -> Result<Element> {
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
                ":id": self.id,
                ":name": format!("$.{name}"),
            },
        )?;
        Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    #[cfg(test)]
    pub fn set_updated_at(
        &self,
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
                ":id": self.id,
                ":updated_at": updated_at.format(&Rfc3339).unwrap(),
            },
        )?;
        Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
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
        Ok(Element::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let overpass_data: String = row.get(1)?;
        let tags: String = row.get(2)?;

        Ok(Element {
            id: row.get(0)?,
            overpass_data: serde_json::from_str(&overpass_data).unwrap(),
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use time::{macros::datetime, OffsetDateTime};

    use crate::{service::overpass::OverpassElement, test::mock_conn, Result};

    use super::Element;

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?;
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?;
        Element::insert(&OverpassElement::mock(2), &conn)?;
        Element::insert(&OverpassElement::mock(3), &conn)?;
        let elements = Element::select_all(None, &conn)?;
        assert_eq!(3, elements.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&datetime!(2023-10-01 00:00 UTC), &conn)?;
        Element::insert(&OverpassElement::mock(2), &conn)?
            .set_updated_at(&datetime!(2023-10-02 00:00 UTC), &conn)?;
        let elements =
            Element::select_updated_since(&datetime!(2023-10-01 00:00 UTC), None, &conn)?;
        assert_eq!(1, elements.len());
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
    fn set_overpass_json() -> Result<()> {
        let conn = mock_conn();
        let override_overpass_json = OverpassElement::mock(2);
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .set_overpass_data(&override_overpass_json, &conn)?;
        assert_eq!(override_overpass_json, element.overpass_data);
        Ok(())
    }

    // #[test]
    // fn set_tags() -> Result<()> {
    //     let conn = mock_conn();
    //     let element = Element::insert(&OverpassElement::mock(1), &conn)?;
    //     let mut tags: HashMap<String, Value> = HashMap::new();
    //     let tag_name = "foo";
    //     let tag_value = Value::String("bar".into());
    //     tags.insert(tag_name.into(), tag_value.clone().into());
    //     Element::set_tags(&element.id, &tags, &conn)?;
    //     let element = Element::select_by_id(&element.id, &conn)?.unwrap();
    //     assert_eq!(&tag_value, &element.tags[tag_name],);
    //     Ok(())
    // }

    #[test]
    fn insert_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let tag_value = "bar";
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .insert_tag(tag_name, tag_value, &conn)?;
        assert_eq!(tag_value, element.tags[tag_name].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn delete_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let element = Element::insert(&OverpassElement::mock(1), &conn)?
            .insert_tag(tag_name, "bar", &conn)?
            .delete_tag(tag_name, &conn)?;
        assert!(!element.tags.contains_key(tag_name));
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
