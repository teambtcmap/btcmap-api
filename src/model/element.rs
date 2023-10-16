use std::thread::sleep;
use std::time::Duration;

use rusqlite::named_params;

use rusqlite::Connection;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;

use super::OverpassElement;

use crate::Result;

pub struct Element {
    pub id: String,
    pub osm_json: OverpassElement,
    pub tags: Map<String, Value>,
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

        sleep(Duration::from_millis(1));

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

pub static SELECT_ALL: &str = r#"
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

pub static SELECT_ALL_MAPPER: fn(&Row) -> rusqlite::Result<Element> = full_mapper();

pub static SELECT_NOT_DELETED: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM element
    WHERE deleted_at = ''
    ORDER BY updated_at
"#;

pub static SELECT_NOT_DELETED_MAPPER: fn(&Row) -> rusqlite::Result<Element> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
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

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> rusqlite::Result<Element> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM element
    WHERE updated_at > :updated_since
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> rusqlite::Result<Element> = full_mapper();

pub static UPDATE_TAGS: &str = r#"
    UPDATE element
    SET tags = :tags
    WHERE id = :element_id
"#;

pub static UPDATE_DELETED_AT: &str = r#"
    UPDATE element
    SET deleted_at = :deleted_at
    WHERE id = :id
"#;

pub static UPDATE_OSM_JSON: &str = r#"
    UPDATE element
    SET osm_json = :osm_json
    WHERE id = :id
"#;

pub static MARK_AS_DELETED: &str = r#"
    UPDATE element
    SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ')
    WHERE id = :id
"#;

pub static INSERT_TAG: &str = r#"
    UPDATE element
    SET tags = json_set(tags, :tag_name, :tag_value)
    WHERE id = :element_id
"#;

pub static DELETE_TAG: &str = r#"
    UPDATE element
    SET tags = json_remove(tags, :tag_name)
    WHERE id = :element_id
"#;

const fn full_mapper() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let osm_json: String = row.get(1)?;
        let osm_json: OverpassElement = serde_json::from_str(&osm_json).unwrap();

        let tags: String = row.get(2)?;
        let tags: Map<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

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
    use crate::{command::db, model::OverpassElement, Result};

    use super::Element;

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        Element::insert(&OverpassElement::mock(), &conn)?;
        Ok(())
    }
}
