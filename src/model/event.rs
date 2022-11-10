use rusqlite::Result;
use rusqlite::Row;

pub struct Event {
    pub id: i64,
    pub date: String,
    pub element_id: String,
    pub r#type: String,
    pub user_id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO event (
        date, 
        element_id, 
        type, 
        user_id
    ) VALUES (
        :date,
        :element_id,
        :type,
        :user_id
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        ROWID,
        date,
        element_id,
        type,
        user_id,
        created_at,
        updated_at,
        deleted_at
    FROM event
    ORDER BY updated_at
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        ROWID,
        date,
        element_id,
        type,
        user_id,
        created_at,
        updated_at,
        deleted_at
    FROM event
    WHERE ROWID = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        ROWID,
        date,
        element_id,
        type,
        user_id,
        created_at,
        updated_at,
        deleted_at
    FROM event
    WHERE updated_at > :updated_since
    ORDER BY updated_at
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Event> = full_mapper();

const fn full_mapper() -> fn(&Row) -> Result<Event> {
    |row: &Row| -> Result<Event> {
        Ok(Event {
            id: row.get(0)?,
            date: row.get(1)?,
            element_id: row.get(2)?,
            r#type: row.get(3)?,
            user_id: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}
