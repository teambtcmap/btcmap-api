use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "place_report";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    PlaceId,
    OriginId,
    Type,
    ExtraFields,
    TicketUrl,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    DeletedAt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceReport {
    pub id: i64,
    pub place_id: i64,
    pub origin_id: i64,
    pub r#type: String,
    pub extra_fields: Map<String, Value>,
    pub ticket_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub closed_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

impl PlaceReport {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::PlaceId,
                Columns::OriginId,
                Columns::Type,
                Columns::ExtraFields,
                Columns::TicketUrl,
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
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Self {
                id: row.get(Columns::Id.as_ref())?,
                place_id: row.get(Columns::PlaceId.as_ref())?,
                origin_id: row.get(Columns::OriginId.as_ref())?,
                r#type: row.get(Columns::Type.as_ref())?,
                extra_fields,
                ticket_url: row.get(Columns::TicketUrl.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                closed_at: row.get(Columns::ClosedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
