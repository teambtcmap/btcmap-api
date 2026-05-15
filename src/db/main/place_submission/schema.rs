use super::vendor;
use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;
use url::Url;

pub const TABLE_NAME: &str = "place_submission";

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

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Origin => "origin",
            Columns::ExternalId => "external_id",
            Columns::Lat => "lat",
            Columns::Lon => "lon",
            Columns::Category => "category",
            Columns::Name => "name",
            Columns::ExtraFields => "extra_fields",
            Columns::TicketUrl => "ticket_url",
            Columns::Revoked => "revoked",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::ClosedAt => "closed_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
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
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            let extra_fields: String = row.get(Columns::ExtraFields.as_str())?;
            let extra_fields = serde_json::from_str(&extra_fields).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Self {
                id: row.get(Columns::Id.as_str())?,
                origin: row.get(Columns::Origin.as_str())?,
                external_id: row.get(Columns::ExternalId.as_str())?,
                lat: row.get(Columns::Lat.as_str())?,
                lon: row.get(Columns::Lon.as_str())?,
                category: row.get(Columns::Category.as_str())?,
                name: row.get(Columns::Name.as_str())?,
                extra_fields,
                ticket_url: row.get(Columns::TicketUrl.as_str())?,
                revoked: row.get(Columns::Revoked.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                closed_at: row.get(Columns::ClosedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }

    pub fn icon(&self) -> String {
        vendor::icon(&self.origin, &self.category)
    }

    pub fn description(&self) -> Option<String> {
        self.extra_fields
            .get("description")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn address(&self) -> Option<String> {
        self.extra_fields
            .get("address")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn opening_hours(&self) -> Option<String> {
        self.extra_fields
            .get("opening_hours")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn phone(&self) -> Option<String> {
        self.extra_fields
            .get("phone")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn website(&self) -> Option<String> {
        let key = "website";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn twitter(&self) -> Option<String> {
        let key = "twitter";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn facebook(&self) -> Option<String> {
        let key = "facebook";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn instagram(&self) -> Option<String> {
        let key = "instagram";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn line(&self) -> Option<String> {
        let key = "line";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn email(&self) -> Option<String> {
        self.extra_fields
            .get("email")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn image(&self) -> Option<String> {
        self.extra_fields
            .get("icon_url")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn payment_provider(&self) -> Option<String> {
        vendor::get(&self.origin)
            .and_then(|vendor| vendor.payment_provider)
            .map(Into::into)
    }
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(url) => url.scheme() == "http" || url.scheme() == "https",
        Err(_) => false,
    }
}
