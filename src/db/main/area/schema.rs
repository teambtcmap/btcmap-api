use crate::Result;
use geojson::{GeoJson, Geometry};
use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "area";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Alias,
    BboxWest,
    BboxSouth,
    BboxEast,
    BboxNorth,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(PartialEq)]
pub struct Area {
    pub id: i64,
    pub alias: String,
    pub bbox_west: f64,
    pub bbox_south: f64,
    pub bbox_east: f64,
    pub bbox_north: f64,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Area {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Alias,
                Columns::BboxWest,
                Columns::BboxSouth,
                Columns::BboxEast,
                Columns::BboxNorth,
                Columns::Tags,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Area> {
        |row: &Row| -> rusqlite::Result<Area> {
            let tags: String = row.get(Columns::Tags.as_ref())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Area {
                id: row.get(Columns::Id.as_ref())?,
                alias: row.get(Columns::Alias.as_ref())?,
                bbox_west: row.get(Columns::BboxWest.as_ref())?,
                bbox_south: row.get(Columns::BboxSouth.as_ref())?,
                bbox_east: row.get(Columns::BboxEast.as_ref())?,
                bbox_north: row.get(Columns::BboxNorth.as_ref())?,
                tags,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }

    pub fn name(&self) -> String {
        self.tags
            .get("name")
            .map(|it| it.as_str().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }

    pub fn alias(&self) -> String {
        self.tags
            .get("url_alias")
            .map(|it| it.as_str().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }

    pub fn geo_json(&self) -> Result<GeoJson> {
        let geo_json = self.tags["geo_json"].clone();
        let geo_json: GeoJson = serde_json::to_string(&geo_json)?.parse()?;
        Ok(geo_json)
    }

    pub fn geo_json_geometries(&self) -> Result<Vec<Geometry>> {
        let mut geometries: Vec<Geometry> = vec![];
        let geo_json = self.tags["geo_json"].clone();
        let geo_json: GeoJson = serde_json::to_string(&geo_json)?.parse()?;
        match geo_json {
            GeoJson::FeatureCollection(v) => {
                for feature in v.features {
                    if let Some(v) = feature.geometry {
                        geometries.push(v);
                    }
                }
            }
            GeoJson::Feature(v) => {
                if let Some(v) = v.geometry {
                    geometries.push(v);
                }
            }
            GeoJson::Geometry(v) => geometries.push(v),
        };
        Ok(geometries)
    }

    #[cfg(test)]
    pub fn mock_tags() -> Map<String, Value> {
        let mut tags = Map::new();
        tags.insert(
            "geo_json".into(),
            serde_json::to_value(GeoJson::Feature(geojson::Feature::default())).unwrap(),
        );
        tags.insert("url_alias".into(), Value::String("alias".into()));
        tags
    }
}

#[cfg(test)]
mod test {
    use super::{Area, Columns};

    #[test]
    fn columns_as_ref() {
        assert_eq!(Columns::Id.as_ref(), "id");
        assert_eq!(Columns::Alias.as_ref(), "alias");
        assert_eq!(Columns::BboxWest.as_ref(), "bbox_west");
        assert_eq!(Columns::BboxSouth.as_ref(), "bbox_south");
        assert_eq!(Columns::BboxEast.as_ref(), "bbox_east");
        assert_eq!(Columns::BboxNorth.as_ref(), "bbox_north");
        assert_eq!(Columns::Tags.as_ref(), "tags");
        assert_eq!(Columns::CreatedAt.as_ref(), "created_at");
        assert_eq!(Columns::UpdatedAt.as_ref(), "updated_at");
        assert_eq!(Columns::DeletedAt.as_ref(), "deleted_at");
    }

    #[test]
    fn projection_contains_all_columns() {
        let projection = Area::projection();
        assert!(projection.contains("id"));
        assert!(projection.contains("alias"));
        assert!(projection.contains("bbox_west"));
        assert!(projection.contains("bbox_south"));
        assert!(projection.contains("bbox_east"));
        assert!(projection.contains("bbox_north"));
        assert!(projection.contains("tags"));
        assert!(projection.contains("created_at"));
        assert!(projection.contains("updated_at"));
        assert!(projection.contains("deleted_at"));
    }
}
