use rusqlite::Row;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "og";

pub enum Columns {
    ElementId,
    Version,
    ImageData,
    CreatedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::ElementId => "element_id",
            Columns::Version => "version",
            Columns::ImageData => "image_data",
            Columns::CreatedAt => "created_at",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct OgImage {
    pub element_id: i64,
    pub version: i64,
    pub image_data: Vec<u8>,
    pub created_at: time::OffsetDateTime,
}

impl OgImage {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::ElementId,
                Columns::Version,
                Columns::ImageData,
                Columns::CreatedAt,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            Ok(Self {
                element_id: row.get(Columns::ElementId.as_str())?,
                version: row.get(Columns::Version.as_str())?,
                image_data: row.get(Columns::ImageData.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
            })
        }
    }
}
