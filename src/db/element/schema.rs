use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;

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
}
