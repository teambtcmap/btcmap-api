use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;

pub struct Report {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

pub static INSERT: &str = r#"
    INSERT INTO report (
        area_id,
        date,
        tags
    ) VALUES (
        :area_id,
        :date,
        :tags
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report
    ORDER BY updated_at
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Report> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report
    WHERE updated_at > :updated_since
    ORDER BY updated_at
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Report> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        id,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report
    WHERE ROWID = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Report> = full_mapper();

pub static SELECT_BY_AREA_ID_AND_DATE: &str = r#"
    SELECT
        id,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report
    WHERE area_id = :area_id AND date = :date
"#;

pub static SELECT_BY_AREA_ID_AND_DATE_MAPPER: fn(&Row) -> Result<Report> = full_mapper();

const fn full_mapper() -> fn(&Row) -> Result<Report> {
    |row: &Row| -> Result<Report> {
        let tags: String = row.get(3)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Report {
            id: row.get(0)?,
            area_id: row.get(1)?,
            date: row.get(2)?,
            tags: tags,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}
