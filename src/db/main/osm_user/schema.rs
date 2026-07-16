use crate::service::osm::EditingApiUser;
use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const NAME: &str = "osm_user";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    OsmData,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
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
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::OsmData,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            let osm_data: String = row.get(Columns::OsmData.as_ref())?;
            let tags: String = row.get(Columns::Tags.as_ref())?;
            let osm_data = serde_json::from_str(&osm_data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Self {
                id: row.get(Columns::Id.as_ref())?,
                osm_data,
                tags,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Columns, OsmUser};

    #[test]
    fn columns_as_ref() {
        assert_eq!(Columns::Id.as_ref(), "id");
        assert_eq!(Columns::OsmData.as_ref(), "osm_data");
        assert_eq!(Columns::Tags.as_ref(), "tags");
        assert_eq!(Columns::CreatedAt.as_ref(), "created_at");
        assert_eq!(Columns::UpdatedAt.as_ref(), "updated_at");
        assert_eq!(Columns::DeletedAt.as_ref(), "deleted_at");
    }

    #[test]
    fn projection_contains_all_columns() {
        let projection = OsmUser::projection();
        assert!(projection.contains("id"));
        assert!(projection.contains("osm_data"));
        assert!(projection.contains("tags"));
        assert!(projection.contains("created_at"));
        assert!(projection.contains("updated_at"));
        assert!(projection.contains("deleted_at"));
    }
}
