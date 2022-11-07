use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;

pub struct Area {
    pub id: String,
    pub name: String,
    pub area_type: String,
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO area (
        id,
        name,
        type,
        min_lon,
        min_lat,
        max_lon,
        max_lat
    )
    VALUES (
        :id,
        '',
        '',
        0,
        0,
        0,
        0
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        name,
        type,
        min_lon,
        min_lat,
        max_lon,
        max_lat,
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
        name,
        type,
        min_lon,
        min_lat,
        max_lon,
        max_lat,
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
        name,
        type,
        min_lon,
        min_lat,
        max_lon,
        max_lat,
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
        let tags: String = row.get(7)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Area {
            id: row.get(0)?,
            name: row.get(1)?,
            area_type: row.get(2)?,
            min_lon: row.get(3)?,
            min_lat: row.get(4)?,
            max_lon: row.get(5)?,
            max_lat: row.get(6)?,
            tags: tags,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            deleted_at: row.get(10)?,
        })
    }
}
