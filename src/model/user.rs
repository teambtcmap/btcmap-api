use rusqlite::Result;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;

use super::OsmUserJson;

pub struct User {
    pub id: i32,
    pub osm_json: OsmUserJson,
    pub tags: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO user (
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
    FROM user
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<User> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM user
    WHERE id = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<User> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM user
    WHERE updated_at > :updated_since
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<User> = full_mapper();

pub static UPDATE_TAGS: &str = r#"
    UPDATE user
    SET tags = :tags
    WHERE id = :user_id
"#;

pub static UPDATE_OSM_JSON: &str = r#"
    UPDATE user
    SET osm_json = :osm_json
    WHERE id = :id
"#;

const fn full_mapper() -> fn(&Row) -> Result<User> {
    |row: &Row| -> Result<User> {
        let osm_json: String = row.get(1)?;
        let osm_json: OsmUserJson = serde_json::from_str(&osm_json).unwrap();

        let tags: String = row.get(2)?;
        let tags: Map<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(User {
            id: row.get(0)?,
            osm_json,
            tags,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}
