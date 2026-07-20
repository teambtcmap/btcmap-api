use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "place_submission";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Origin,
    ExternalId,
    Lat,
    Lon,
    Category,
    Name,
    ExtraFields,
    TicketUrl,
    Revoked,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    DeletedAt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceSubmission {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    pub extra_fields: Map<String, Value>,
    pub ticket_url: Option<String>,
    pub revoked: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub closed_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, PartialEq)]
pub struct OriginSubmissionCounts {
    pub origin: String,
    pub total: i64,
    pub pending: i64,
    pub revoked: i64,
}

impl PlaceSubmission {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Origin,
                Columns::ExternalId,
                Columns::Lat,
                Columns::Lon,
                Columns::Category,
                Columns::Name,
                Columns::ExtraFields,
                Columns::TicketUrl,
                Columns::Revoked,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::ClosedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            let extra_fields: String = row.get(Columns::ExtraFields.as_ref())?;
            let extra_fields = serde_json::from_str(&extra_fields).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Self {
                id: row.get(Columns::Id.as_ref())?,
                origin: row.get(Columns::Origin.as_ref())?,
                external_id: row.get(Columns::ExternalId.as_ref())?,
                lat: row.get(Columns::Lat.as_ref())?,
                lon: row.get(Columns::Lon.as_ref())?,
                category: row.get(Columns::Category.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                extra_fields,
                ticket_url: row.get(Columns::TicketUrl.as_ref())?,
                revoked: row.get(Columns::Revoked.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                closed_at: row.get(Columns::ClosedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
