use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;

pub struct User {
    pub id: i64,
    pub osm_json: Value,
    pub tags: Value,
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
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<User> = full_mapper();

pub static INSERT_TAG: &str = r#"
    UPDATE user
    SET tags = json_set(tags, :tag_name, :tag_value)
    WHERE id = :user_id
"#;

pub static UPDATE_OSM_JSON: &str = r#"
    UPDATE user
    SET osm_json = :osm_json
    WHERE id = :id
"#;

pub static DELETE_TAG: &str = r#"
    UPDATE user
    SET tags = json_remove(tags, :tag_name)
    WHERE id = :user_id
"#;

const fn full_mapper() -> fn(&Row) -> Result<User> {
    |row: &Row| -> Result<User> {
        let osm_json: String = row.get(1)?;
        let osm_json: Value = serde_json::from_str(&osm_json).unwrap_or_default();

        let tags: String = row.get(2)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

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
