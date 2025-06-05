use rusqlite::Row;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "area_element";

pub enum Columns {
    Id,
    AreaId,
    ElementId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::AreaId => "area_id",
            Columns::ElementId => "element_id",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
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
    pub fn projection() -> String {
        [
            Columns::Id,
            Columns::AreaId,
            Columns::ElementId,
            Columns::CreatedAt,
            Columns::UpdatedAt,
            Columns::DeletedAt,
        ]
        .iter()
        .map(Columns::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn mapper() -> fn(&Row) -> rusqlite::Result<AreaElement> {
        |row: &Row| -> rusqlite::Result<AreaElement> {
            Ok(AreaElement {
                id: row.get(0)?,
                area_id: row.get(1)?,
                element_id: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}
