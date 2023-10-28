use std::collections::HashMap;

use rusqlite::named_params;
use rusqlite::OptionalExtension;

use rusqlite::Connection;
use rusqlite::Row;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::service::overpass::OverpassElement;
use crate::Error;
use crate::Result;

#[derive(PartialEq, Debug, Clone)]
pub struct Element {
    pub id: String,
    pub overpass_json: OverpassElement,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Element {
    pub fn insert(overpass_json: &OverpassElement, conn: &Connection) -> Result<Element> {
        let query = r#"
            INSERT INTO element (
                id,
                overpass_json
            ) VALUES (
                :id,
                :overpass_json
            )
        "#;
        conn.execute(
            query,
            named_params! {
                ":id": overpass_json.btcmap_id(),
                ":overpass_json": serde_json::to_string(overpass_json)?,
            },
        )?;
        let element = Element::select_by_id(&overpass_json.btcmap_id(), &conn)?
            .ok_or(Error::DbTableRowNotFound)?;
        Ok(element)
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<Element>> {
        let query = r#"
            SELECT
                id,
                overpass_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM element
            ORDER BY updated_at, id
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Element>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
        limit: Option<i32>,
        conn: &Connection,
    ) -> Result<Vec<Element>> {
        let query = r#"
            SELECT
                id,
                overpass_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM element
            WHERE updated_at > :updated_since
            ORDER BY updated_at, id
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":updated_since": updated_since, ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Element>, _>>()?)
    }

    pub fn select_by_id(id: &str, conn: &Connection) -> Result<Option<Element>> {
        let query = r#"
            SELECT
                id,
                overpass_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM element 
            WHERE id = :id
        "#;

        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn set_tags(id: &str, tags: &HashMap<String, Value>, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE element
            SET tags = json(:tags)
            WHERE id = :id
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":tags": serde_json::to_string(tags)?,
            },
        )?;

        Ok(())
    }

    pub fn set_overpass_json(
        id: &str,
        overpass_json: &OverpassElement,
        conn: &Connection,
    ) -> Result<()> {
        let query = r#"
            UPDATE element
            SET overpass_json = json(:overpass_json)
            WHERE id = :id
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":overpass_json": serde_json::to_string(overpass_json)?,
            },
        )?;

        Ok(())
    }

    pub fn insert_tag(
        id: &str,
        tag_name: &str,
        tag_value: &str,
        conn: &Connection,
    ) -> crate::Result<()> {
        let tag_name = format!("$.{tag_name}");

        let query = r#"
            UPDATE element
            SET tags = json_set(tags, :tag_name, :tag_value)
            WHERE id = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":tag_name": tag_name, ":tag_value": tag_value },
        )?;

        Ok(())
    }

    pub fn delete_tag(id: &str, tag: &str, conn: &Connection) -> Result<()> {
        let tag = format!("$.{tag}");

        let query = r#"
            UPDATE element
            SET tags = json_remove(tags, :tag)
            WHERE id = :id
        "#;

        conn.execute(query, named_params! { ":id": id, ":tag": tag })?;

        Ok(())
    }

    #[cfg(test)]
    pub fn set_updated_at(
        &self,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Element> {
        conn.execute(
            "UPDATE element SET updated_at = :updated_at WHERE id = :id",
            named_params! {
                ":id": self.id,
                ":updated_at": updated_at.format(&Rfc3339).unwrap(),
            },
        )?;
        Ok(Element {
            id: self.id.clone(),
            overpass_json: self.overpass_json.clone(),
            tags: self.tags.clone(),
            created_at: self.created_at.clone(),
            updated_at: updated_at.clone(),
            deleted_at: self.deleted_at.clone(),
        })
    }

    pub fn set_deleted_at(
        id: &str,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<()> {
        let deleted_at = deleted_at.map(|it| it.format(&Rfc3339).unwrap());

        match deleted_at {
            Some(deleted_at) => {
                let query = r#"
                    UPDATE element
                    SET deleted_at = :deleted_at
                    WHERE id = :id
                "#;

                conn.execute(
                    query,
                    named_params! { ":id": id, ":deleted_at": deleted_at },
                )?;
            }
            None => {
                let query = r#"
                    UPDATE element
                    SET deleted_at = NULL
                    WHERE id = :id
                "#;

                conn.execute(query, named_params! { ":id": id })?;
            }
        };

        Ok(())
    }

    pub fn get_btcmap_tag_value_str(&self, name: &str) -> &str {
        self.tags
            .get(name)
            .map(|it| it.as_str().unwrap_or(""))
            .unwrap_or("")
    }

    pub fn get_osm_tag_value(&self, name: &str) -> &str {
        self.overpass_json.get_tag_value(name)
    }

    pub fn generate_android_icon(&self) -> String {
        self.overpass_json.generate_android_icon()
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let osm_json: String = row.get(1)?;
        let osm_json: OverpassElement = serde_json::from_str(&osm_json).unwrap();

        let tags: String = row.get(2)?;
        let tags: HashMap<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Element {
            id: row.get(0)?,
            overpass_json: osm_json,
            tags,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::Value;
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
        let elements = Element::select_updated_since("2023-10-01T00:00:00Z", None, &conn)?;
        assert_eq!(1, elements.len());
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(element, Element::select_by_id(&element.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn set_overpass_json() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?;
        let new_overpass_json = OverpassElement::mock(2);
        Element::set_overpass_json("node:1", &new_overpass_json, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert_eq!(new_overpass_json, element.overpass_json);
        Ok(())
    }

    #[test]
    fn set_tags() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let mut tags: HashMap<String, Value> = HashMap::new();
        let tag_name = "foo";
        let tag_value = Value::String("bar".into());
        tags.insert(tag_name.into(), tag_value.clone().into());
        Element::set_tags(&element.id, &tags, &conn)?;
        let element = Element::select_by_id(&element.id, &conn)?.unwrap();
        assert_eq!(&tag_value, &element.tags[tag_name],);
        Ok(())
    }

    #[test]
    fn insert_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        let tag_value = "bar";
        Element::insert(&OverpassElement::mock(1), &conn)?;
        Element::insert_tag("node:1", tag_name, tag_value, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert_eq!(tag_value, element.tags[tag_name].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn delete_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "foo";
        Element::insert(&OverpassElement::mock(1), &conn)?;
        Element::insert_tag("node:1", tag_name, "bar", &conn)?;
        Element::delete_tag("node:1", tag_name, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert!(!element.tags.contains_key(tag_name));
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let deleted_at = OffsetDateTime::now_utc();
        Element::set_deleted_at(&element.id, Some(deleted_at), &conn)?;
        assert_eq!(
            deleted_at,
            Element::select_by_id(&element.id, &conn)?
                .unwrap()
                .deleted_at
                .unwrap()
        );
        Ok(())
    }
}
