use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "note";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    UserId,
    Lat,
    Lon,
    Text,
    Public,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: i64,
    pub user_id: i64,
    pub lat: f64,
    pub lon: f64,
    pub text: String,
    pub public: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Note {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::UserId,
                Columns::Lat,
                Columns::Lon,
                Columns::Text,
                Columns::Public,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Note> {
        |row| {
            Ok(Note {
                id: row.get(Columns::Id.as_ref())?,
                user_id: row.get(Columns::UserId.as_ref())?,
                lat: row.get(Columns::Lat.as_ref())?,
                lon: row.get(Columns::Lon.as_ref())?,
                text: row.get(Columns::Text.as_ref())?,
                public: row.get(Columns::Public.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
