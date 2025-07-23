use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "boost";

pub enum Columns {
    Id,
    AdminId,
    ElementId,
    DurationDays,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::AdminId => "admin_id",
            Columns::ElementId => "element_id",
            Columns::DurationDays => "duration_days",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub struct Boost {
    pub id: i64,
    pub admin_id: i64,
    pub element_id: i64,
    pub duration_days: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Boost {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::AdminId,
                Columns::ElementId,
                Columns::DurationDays,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            Ok(Self {
                id: row.get(Columns::Id.as_str())?,
                admin_id: row.get(Columns::AdminId.as_str())?,
                element_id: row.get(Columns::ElementId.as_str())?,
                duration_days: row.get(Columns::DurationDays.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::CreatedAt.as_str())?,
                deleted_at: row.get(Columns::CreatedAt.as_str())?,
            })
        }
    }
}
