use geojson::JsonObject;
use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "event";

pub enum Columns {
    Id,
    UserId,
    ElementId,
    Type,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::UserId => "user_id",
            Columns::ElementId => "element_id",
            Columns::Type => "type",
            Columns::Tags => "tags",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
    pub id: i64,
    pub user_id: i64,
    pub element_id: i64,
    pub r#type: String,
    pub tags: JsonObject,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Event {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::UserId,
                Columns::ElementId,
                Columns::Type,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Event> {
        |row| {
            let tags: String = row.get(Columns::Tags.as_str())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Event {
                id: row.get(Columns::Id.as_str())?,
                user_id: row.get(Columns::UserId.as_str())?,
                element_id: row.get(Columns::ElementId.as_str())?,
                r#type: row.get(Columns::Type.as_str())?,
                tags,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }
}
