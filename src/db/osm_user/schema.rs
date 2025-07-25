use crate::service::osm::EditingApiUser;
use rusqlite::Row;
use serde_json::{Map, Value};
use time::OffsetDateTime;

pub const NAME: &str = "osm_user";

pub enum Columns {
    Id,
    OsmData,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::OsmData => "osm_data",
            Columns::Tags => "tags",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct OsmUser {
    pub id: i64,
    pub osm_data: EditingApiUser,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl OsmUser {
    pub fn projection() -> String {
        [
            Columns::Id,
            Columns::OsmData,
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

    pub fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            let osm_data: String = row.get(1)?;
            let tags: String = row.get(2)?;

            Ok(Self {
                id: row.get(0)?,
                osm_data: serde_json::from_str(&osm_data).unwrap(),
                tags: serde_json::from_str(&tags).unwrap(),
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}
