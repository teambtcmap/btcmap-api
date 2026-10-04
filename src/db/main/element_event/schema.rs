use geojson::JsonObject;
use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "element_event";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
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

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ElementEvent {
    pub id: i64,
    pub user_id: i64,
    pub element_id: i64,
    pub r#type: String,
    pub tags: JsonObject,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl ElementEvent {
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
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<ElementEvent> {
        |row| {
            let tags: String = row.get(Columns::Tags.as_ref())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(ElementEvent {
                id: row.get(Columns::Id.as_ref())?,
                user_id: row.get(Columns::UserId.as_ref())?,
                element_id: row.get(Columns::ElementId.as_ref())?,
                r#type: row.get(Columns::Type.as_ref())?,
                tags,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
