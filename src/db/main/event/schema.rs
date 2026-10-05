use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::types::FromSqlResult;
use rusqlite::types::ToSql;
use rusqlite::types::ToSqlOutput;
use rusqlite::types::ValueRef;
use rusqlite::Row;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
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
    Status,
    SubmittedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

/// Review state of an event. Public `GET /v4/events*` responses always carry it.
/// Newly submitted events start as [`Status::Pending`] unless the submitter is
/// privileged, in which case they go straight to [`Status::Live`].
#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Live,
    Rejected,
}

impl Status {
    /// Every variant, in the order clients are expected to enumerate them.
    pub const ALL: [Status; 3] = [Status::Pending, Status::Live, Status::Rejected];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Live => "live",
            Status::Rejected => "rejected",
        }
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Status::Pending),
            "live" => Ok(Status::Live),
            "rejected" => Ok(Status::Rejected),
            _ => Err(format!("'{}' is not a valid event status", s)),
        }
    }
}

impl FromSql for Status {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str()?.parse().map_err(|e: String| {
            FromSqlError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e,
            )))
        })
    }
}

impl ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Debug, Clone)]
pub struct Event {
    pub id: i64,
    pub area_id: Option<i64>,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub status: Status,
    pub submitted_by: Option<i64>,
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
                Columns::Status,
                Columns::SubmittedBy,
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
                status: row.get(Columns::Status.as_ref())?,
                submitted_by: row.get(Columns::SubmittedBy.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
