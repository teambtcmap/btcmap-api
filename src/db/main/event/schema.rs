use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE: &str = "event";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    AreaId,
    Lat,
    Lon,
    Name,
    Website,
    StartsAt,
    EndsAt,
    CronSchedule,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct Event {
    pub id: i64,
    pub area_id: Option<i64>,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    pub starts_at: Option<OffsetDateTime>,
    pub ends_at: Option<OffsetDateTime>,
    pub cron_schedule: Option<String>,
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
                Columns::AreaId,
                Columns::Lat,
                Columns::Lon,
                Columns::Name,
                Columns::Website,
                Columns::StartsAt,
                Columns::EndsAt,
                Columns::CronSchedule,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            Ok(Event {
                id: row.get(Columns::Id.as_ref())?,
                area_id: row.get(Columns::AreaId.as_ref())?,
                lat: row.get(Columns::Lat.as_ref())?,
                lon: row.get(Columns::Lon.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                website: row.get(Columns::Website.as_ref())?,
                starts_at: row.get(Columns::StartsAt.as_ref())?,
                ends_at: row.get(Columns::EndsAt.as_ref())?,
                cron_schedule: row.get(Columns::CronSchedule.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
