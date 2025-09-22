use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::service::overpass::OverpassElement;

pub const TABLE_NAME: &str = "element";

pub enum Columns {
    Id,
    OverpassData,
    Tags,
    Lat,
    Lon,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::OverpassData => "overpass_data",
            Columns::Tags => "tags",
            Columns::Lat => "lat",
            Columns::Lon => "lon",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub id: i64,
    pub overpass_data: OverpassElement,
    pub tags: Map<String, Value>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Element {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::OverpassData,
                Columns::Tags,
                Columns::Lat,
                Columns::Lon,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Element> {
        |row| {
            let overpass_data: String = row.get(Columns::OverpassData.as_str())?;
            let overpass_data = serde_json::from_str(&overpass_data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            let tags: String = row.get(Columns::Tags.as_str())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Element {
                id: row.get(Columns::Id.as_str())?,
                overpass_data,
                tags,
                lat: row.get(Columns::Lat.as_str())?,
                lon: row.get(Columns::Lon.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }

    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }

    pub fn name(&self) -> String {
        self.overpass_data.tag("name").into()
    }

    pub fn osm_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/{}/{}",
            self.overpass_data.r#type, self.overpass_data.id,
        )
    }

    pub fn lat(&self) -> f64 {
        self.overpass_data.coord().y
    }

    pub fn lon(&self) -> f64 {
        self.overpass_data.coord().x
    }

    pub fn supports_payment_provider(&self, provider: &str) -> bool {
        match &self.overpass_data.tags {
            Some(tags) => match tags.get(&format!("payment:{}", provider)) {
                Some(tag) => tag.is_string() && tag.as_str() == Some("yes"),
                None => false,
            },
            None => false,
        }
    }

    pub fn address(&self) -> Option<String> {
        let mut addr = String::new();
        let housenumber = self.overpass_data.tag("addr:housenumber");
        if !housenumber.is_empty() {
            addr.push_str(housenumber);
            addr.push(' ');
        }
        let street = self.overpass_data.tag("addr:street");
        if !street.is_empty() {
            addr.push_str(street);
            addr.push(' ');
        }
        let city = self.overpass_data.tag("addr:city");
        if !city.is_empty() {
            addr.push_str(city);
            addr.push(' ');
        }
        let postcode = self.overpass_data.tag("addr:postcode");
        if !postcode.is_empty() {
            addr.push_str(postcode);
            addr.push(' ');
        }
        let addr = addr.trim();
        if addr.is_empty() {
            None
        } else {
            Some(addr.to_string())
        }
    }

    pub fn opening_hours(&self) -> Option<String> {
        let res = self.overpass_data.tag("opening_hours");

        if res.is_empty() {
            None
        } else {
            Some(res.to_string())
        }
    }

    pub fn comment_count(&self) -> i64 {
        if self.tags.contains_key("comments") {
            self.tags["comments"].as_i64().unwrap_or(0)
        } else {
            0
        }
    }

    pub fn icon(&self, default: &str) -> String {
        self.tags
            .get("icon:android")
            .unwrap_or(&Value::String(default.into()))
            .as_str()
            .unwrap_or(default)
            .to_string()
    }

    pub fn verified_at(&self) -> Option<OffsetDateTime> {
        self.overpass_data.verification_date()
    }

    pub fn osm_id(&self) -> String {
        self.overpass_data.btcmap_id()
    }

    pub fn boosted_until(&self) -> Option<OffsetDateTime> {
        match self.tags.get("boost:expires") {
            Some(boost_expires) => match boost_expires.as_str() {
                Some(boost_expires) => match OffsetDateTime::parse(boost_expires, &Rfc3339) {
                    Ok(boost_expires) => Some(boost_expires),
                    Err(_) => None,
                },
                None => None,
            },
            None => None,
        }
    }

    pub fn phone(&self) -> Option<String> {
        let Some(osm_tags) = &self.overpass_data.tags else {
            return None;
        };

        let variants = vec!["phone", "contact:phone"];

        for variant in variants {
            if osm_tags.contains_key(variant) && osm_tags[variant].is_string() {
                return Some(osm_tags[variant].as_str().unwrap_or("").to_string());
            }
        }

        return None;
    }
}
