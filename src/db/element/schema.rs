use crate::element::Element;
use rusqlite::Row;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "element";

pub enum Columns {
    Id,
    OverpassData,
    Tags,
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
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

// pub struct Element {
//     pub id: i64,
//     pub overpass_data: OverpassElement,
//     pub tags: Map<String, Value>,
//     pub created_at: OffsetDateTime,
//     pub updated_at: OffsetDateTime,
//     pub deleted_at: Option<OffsetDateTime>,
// }

impl Element {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::OverpassData,
                Columns::Tags,
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
            let overpass_data: String = row.get(1)?;
            let overpass_data = serde_json::from_str(&overpass_data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            let tags: String = row.get(2)?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Element {
                id: row.get(0)?,
                overpass_data,
                tags,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}
