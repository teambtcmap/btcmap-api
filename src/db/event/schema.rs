use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "event";

pub enum Columns {
    Id,
    Lat,
    Lon,
    Name,
    Website,
    StartsAt,
    EndsAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Lat => "lat",
            Columns::Lon => "lon",
            Columns::Name => "name",
            Columns::Website => "website",
            Columns::StartsAt => "starts_at",
            Columns::EndsAt => "ends_at",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct Event {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Event {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Lat,
                Columns::Lon,
                Columns::Name,
                Columns::Website,
                Columns::StartsAt,
                Columns::EndsAt,
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
        |row: &Row| -> rusqlite::Result<Self> {
            Ok(Event {
                id: row.get(Columns::Id.as_str())?,
                lat: row.get(Columns::Lat.as_str())?,
                lon: row.get(Columns::Lon.as_str())?,
                name: row.get(Columns::Name.as_str())?,
                website: row.get(Columns::Website.as_str())?,
                starts_at: row.get(Columns::StartsAt.as_str())?,
                ends_at: row.get(Columns::EndsAt.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }
}
