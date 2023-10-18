use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

use rusqlite::named_params;
use rusqlite::OptionalExtension;

use rusqlite::Connection;
use rusqlite::Row;
use serde_json::Value;

use super::OverpassElement;

use crate::Result;

pub struct Element {
    pub id: String,
    pub osm_json: OverpassElement,
    pub tags: HashMap<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Element {
    pub fn insert(overpass_json: &OverpassElement, conn: &Connection) -> Result<()> {
        let query = r#"
            INSERT INTO element (
                id,
                osm_json
            ) VALUES (
                :id,
                :osm_json
            )
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": overpass_json.btcmap_id(),
                ":osm_json": serde_json::to_string(overpass_json)?,
            },
        )?;

        sleep(Duration::from_millis(10));

        Ok(())
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<Element>> {
        let query = r#"
            SELECT
                id,
                osm_json,
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
                full_mapper(),
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
                osm_json,
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
                full_mapper(),
            )?
            .collect::<Result<Vec<Element>, _>>()?)
    }

    pub fn select_by_id(id: &str, conn: &Connection) -> Result<Option<Element>> {
        let query = r#"
            SELECT
                id,
                osm_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM element 
            WHERE id = :id
        "#;

        Ok(conn
            .query_row(query, named_params! { ":id": id }, full_mapper())
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
            SET osm_json = json(:overpass_json)
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

    pub fn set_deleted_at(id: &str, deleted_at: &str, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE element
            SET deleted_at = :deleted_at
            WHERE id = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":deleted_at": deleted_at },
        )?;

        Ok(())
    }

    pub fn get_btcmap_tag_value_str(&self, name: &str) -> &str {
        self.tags
            .get(name)
            .map(|it| it.as_str().unwrap_or(""))
            .unwrap_or("")
    }

    pub fn get_osm_tag_value(&self, name: &str) -> &str {
        self.osm_json.get_tag_value(name)
    }

    pub fn generate_android_icon(&self) -> String {
        self.osm_json.generate_android_icon()
    }

    #[cfg(test)]
    pub fn set_updated_at(id: &str, updated_at: &str, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE element SET updated_at = :updated_at WHERE id = :id
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at,
            },
        )?;

        Ok(())
    }
}

const fn full_mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let osm_json: String = row.get(1)?;
        let osm_json: OverpassElement = serde_json::from_str(&osm_json).unwrap();

        let tags: String = row.get(2)?;
        let tags: HashMap<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Element {
            id: row.get(0)?,
            osm_json,
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

    use crate::{command::db, model::OverpassElement, Result};

    use super::Element;

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        Element::insert(&OverpassElement::mock(), &conn)?;
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        Element::insert(
            &OverpassElement {
                id: 1,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        Element::insert(
            &OverpassElement {
                id: 2,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        Element::insert(
            &OverpassElement {
                id: 3,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        let elements = Element::select_all(None, &conn)?;
        assert_eq!(3, elements.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = db::setup_connection()?;
        Element::insert(
            &OverpassElement {
                id: 1,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        Element::set_updated_at("node:1", "2023-10-01", &conn)?;
        Element::insert(
            &OverpassElement {
                id: 2,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        Element::set_updated_at("node:2", "2023-10-02", &conn)?;
        let elements = Element::select_updated_since("2023-10-01", None, &conn)?;
        assert_eq!(1, elements.len());
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = db::setup_connection()?;
        let element = OverpassElement {
            id: 1,
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;
        assert_eq!(
            element.btcmap_id(),
            Element::select_by_id(&element.btcmap_id(), &conn)?
                .unwrap()
                .id
        );
        Ok(())
    }

    #[test]
    fn set_overpass_json() -> Result<()> {
        let conn = db::setup_connection()?;
        let element = OverpassElement {
            id: 1,
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;
        let element = OverpassElement {
            id: 2,
            ..OverpassElement::mock()
        };
        Element::set_overpass_json("node:1", &element, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert_eq!(2, element.osm_json.id);
        Ok(())
    }

    #[test]
    fn set_tags() -> Result<()> {
        let conn = db::setup_connection()?;
        let element = OverpassElement {
            id: 1,
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;
        let mut tags: HashMap<String, Value> = HashMap::new();
        let tag_name = "foo";
        let tag_value = Value::String("bar".into());
        tags.insert(tag_name.into(), tag_value.clone().into());
        Element::set_tags(&element.btcmap_id(), &tags, &conn)?;
        let element = Element::select_by_id(&element.btcmap_id(), &conn)?.unwrap();
        assert_eq!(&tag_value, &element.tags[tag_name],);
        Ok(())
    }

    #[test]
    fn insert_tag() -> Result<()> {
        let conn = db::setup_connection()?;
        let tag_name = "foo";
        let tag_value = "bar";
        Element::insert(&OverpassElement::mock(), &conn)?;
        Element::insert_tag("node:1", tag_name, tag_value, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert_eq!(tag_value, element.tags[tag_name].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn delete_tag() -> Result<()> {
        let conn = db::setup_connection()?;
        let tag_name = "foo";
        Element::insert(&OverpassElement::mock(), &conn)?;
        Element::insert_tag("node:1", tag_name, "bar", &conn)?;
        Element::delete_tag("node:1", tag_name, &conn)?;
        let element = Element::select_by_id("node:1", &conn)?.unwrap();
        assert!(!element.tags.contains_key(tag_name));
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = db::setup_connection()?;
        let element = OverpassElement {
            id: 1,
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;
        let deleted_at = "2023-01-01";
        Element::set_deleted_at(&element.btcmap_id(), &deleted_at, &conn)?;
        assert_eq!(
            deleted_at,
            Element::select_by_id(&element.btcmap_id(), &conn)?
                .unwrap()
                .deleted_at
        );
        Ok(())
    }
}
