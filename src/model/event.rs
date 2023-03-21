use rusqlite::Result;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;

pub struct Event {
    pub id: i64,
    pub user_id: i64,
    pub element_id: String,
    pub r#type: String,
    pub tags: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO event (
        user_id,
        element_id, 
        type
    ) VALUES (
        :user_id,
        :element_id,
        :type
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        user_id,
        element_id,
        type,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM event
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        id,
        user_id,
        element_id,
        type,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM event
    WHERE id = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        user_id,
        element_id,
        type,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM event
    WHERE updated_at > :updated_since
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

pub static UPDATE_TAGS: &str = r#"
    UPDATE event
    SET tags = :tags
    WHERE id = :event_id
"#;

const fn full_mapper() -> fn(&Row) -> Result<Event> {
    |row: &Row| -> Result<Event> {
        let tags: String = row.get(4)?;
        let tags: Map<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Event {
            id: row.get(0)?,
            user_id: row.get(1)?,
            element_id: row.get(2)?,
            r#type: row.get(3)?,
            tags: tags,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}
