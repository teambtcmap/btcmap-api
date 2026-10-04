use rusqlite::types::{FromSql, FromSqlError, ToSql, ToSqlOutput, ValueRef};
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
    RevocationAction,
    RevocationProcessedAt,
    SubmittedBy,
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
    /// Which Gitea action the revocation sync decided on, recorded the first time
    /// it sees the submission revoked. A retry re-applies this instead of
    /// inferring it again from a ticket state the earlier attempt may have
    /// changed itself.
    pub revocation_action: Option<RevocationAction>,
    /// Set once every Gitea call for the revocation has succeeded. The sync query
    /// ignores submissions that have it.
    pub revocation_processed_at: Option<OffsetDateTime>,
    pub submitted_by: Option<i64>,
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
                Columns::RevocationAction,
                Columns::RevocationProcessedAt,
                Columns::SubmittedBy,
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
                revocation_action: row.get(Columns::RevocationAction.as_ref())?,
                revocation_processed_at: row.get(Columns::RevocationProcessedAt.as_ref())?,
                submitted_by: row.get(Columns::SubmittedBy.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                closed_at: row.get(Columns::ClosedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}

/// What the revocation sync does with a revoked submission's Gitea ticket.
#[derive(Clone, Copy, Debug, PartialEq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum RevocationAction {
    /// The ticket was still open when the submission was revoked, so it had not
    /// been processed yet: the revocation cancels it.
    Close,
    /// The ticket had already been closed, which means the place was processed:
    /// the revocation has to be raised as a removal request.
    Reopen,
}

impl TryFrom<&str> for RevocationAction {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "close" => Ok(RevocationAction::Close),
            "reopen" => Ok(RevocationAction::Reopen),
            _ => Err(format!("Unknown revocation action: {}", value).into()),
        }
    }
}

impl FromSql for RevocationAction {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_str()
            .and_then(|s| RevocationAction::try_from(s).map_err(FromSqlError::Other))
    }
}

impl ToSql for RevocationAction {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn revocation_action_round_trips_through_storage() {
        for action in [RevocationAction::Close, RevocationAction::Reopen] {
            let stored = action.to_string();
            assert_eq!(action, RevocationAction::try_from(stored.as_str()).unwrap());
        }
    }

    #[test]
    fn revocation_action_rejects_unknown_stored_values() {
        assert!(RevocationAction::try_from("cancel").is_err());
    }
}
