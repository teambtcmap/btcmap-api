use crate::Result;
use geojson::{GeoJson, Geometry};
use rusqlite::Row;
use serde_json::{Map, Value};
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "area";

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

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Alias => "alias",
            Columns::BboxWest => "bbox_west",
            Columns::BboxSouth => "bbox_south",
            Columns::BboxEast => "bbox_east",
            Columns::BboxNorth => "bbox_north",
            Columns::Tags => "tags",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
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
    pub fn projection() -> String {
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
        .map(Columns::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn mapper() -> fn(&Row) -> rusqlite::Result<Area> {
        |row: &Row| -> rusqlite::Result<Area> {
            let tags: String = row.get(Columns::Tags.as_str())?;
            Ok(Area {
                id: row.get(Columns::Id.as_str())?,
                alias: row.get(Columns::Alias.as_str())?,
                bbox_west: row.get(Columns::BboxWest.as_str())?,
                bbox_south: row.get(Columns::BboxSouth.as_str())?,
                bbox_east: row.get(Columns::BboxEast.as_str())?,
                bbox_north: row.get(Columns::BboxNorth.as_str())?,
                tags: serde_json::from_str(&tags).unwrap(),
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
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
            GeoJson::Feature(geojson::Feature::default()).into(),
        );
        tags.insert("url_alias".into(), Value::String("alias".into()));
        tags
    }
}
