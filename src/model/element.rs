use rusqlite::Result;
use rusqlite::Row;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Element {
    pub id: String,
    pub osm_json: Value,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO element (
        id,
        osm_json
    ) VALUES (
        :id,
        :osm_json
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM element
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Element> = full_mapper();

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

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Element> = full_mapper();

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

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Element> = full_mapper();

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
    SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ')
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

const fn full_mapper() -> fn(&Row) -> Result<Element> {
    |row: &Row| -> Result<Element> {
        let osm_json: String = row.get(1)?;
        let osm_json: Value = serde_json::from_str(&osm_json).unwrap_or_default();

        let tags: String = row.get(2)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

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
