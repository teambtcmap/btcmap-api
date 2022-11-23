use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;

pub struct Area {
    pub id: String,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO area (
        id
    )
    VALUES (
        :id
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    ORDER BY updated_at
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    WHERE id = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    WHERE updated_at > :updated_since
    ORDER BY updated_at
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static INSERT_TAG: &str = r#"
    UPDATE area
    SET tags = json_set(tags, :tag_name, :tag_value)
    WHERE id = :area_id
"#;

pub static DELETE_TAG: &str = r#"
    UPDATE area
    SET tags = json_remove(tags, :tag_name)
    where id = :area_id
"#;

const fn full_mapper() -> fn(&Row) -> Result<Area> {
    |row: &Row| -> Result<Area> {
        let tags: String = row.get(1)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Area {
            id: row.get(0)?,
            tags: tags,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}
