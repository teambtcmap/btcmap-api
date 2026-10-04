use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "area";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    AreaId,
    Type,
    ImageData,
    Width,
    Height,
    SizeBytes,
    CreatedAt,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AreaImage {
    pub id: i64,
    pub area_id: i64,
    pub r#type: String,
    pub image_data: Vec<u8>,
    pub width: i64,
    pub height: i64,
    pub size_bytes: i64,
    pub created_at: OffsetDateTime,
}

impl AreaImage {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::AreaId,
                Columns::Type,
                Columns::ImageData,
                Columns::Width,
                Columns::Height,
                Columns::SizeBytes,
                Columns::CreatedAt,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            Ok(Self {
                id: row.get(Columns::Id.as_ref())?,
                area_id: row.get(Columns::AreaId.as_ref())?,
                r#type: row.get(Columns::Type.as_ref())?,
                image_data: row.get(Columns::ImageData.as_ref())?,
                width: row.get(Columns::Width.as_ref())?,
                height: row.get(Columns::Height.as_ref())?,
                size_bytes: row.get(Columns::SizeBytes.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
            })
        }
    }
}
