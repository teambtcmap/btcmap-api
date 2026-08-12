use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "area_element";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    AreaId,
    ElementId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Debug)]
pub struct AreaElement {
    pub id: i64,
    pub area_id: i64,
    pub element_id: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl AreaElement {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::AreaId,
                Columns::ElementId,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<AreaElement> {
        |row: &Row| -> rusqlite::Result<AreaElement> {
            Ok(AreaElement {
                id: row.get(Columns::Id.as_ref())?,
                area_id: row.get(Columns::AreaId.as_ref())?,
                element_id: row.get(Columns::ElementId.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
