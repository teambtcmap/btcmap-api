use geojson::JsonObject;
use rusqlite::Row;
use std::sync::OnceLock;
use time::{format_description::well_known::Rfc3339, Date, OffsetDateTime};

pub const TABLE_NAME: &str = "report";

pub enum Columns {
    Id,
    AreaId,
    Date,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::AreaId => "area_id",
            Columns::Date => "date",
            Columns::Tags => "tags",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub id: i64,
    pub area_id: i64,
    pub date: Date,
    pub tags: JsonObject,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Report {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::AreaId,
                Columns::Date,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Report> {
        |row: &_| {
            let tags: String = row.get(Columns::Tags.as_str())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Report {
                id: row.get(Columns::Id.as_str())?,
                area_id: row.get(Columns::AreaId.as_str())?,
                date: row.get(Columns::Date.as_str())?,
                tags,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }

    pub fn total_elements(&self) -> i64 {
        self.tags
            .get("total_elements")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn total_atms(&self) -> i64 {
        self.tags
            .get("total_atms")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn up_to_date_elements(&self) -> i64 {
        self.tags
            .get("up_to_date_elements")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn days_since_verified(&self) -> i64 {
        let now = OffsetDateTime::now_utc();
        let now_str = now.format(&Rfc3339).unwrap();
        match self.tags.get("avg_verification_date") {
            Some(date) => {
                let date =
                    OffsetDateTime::parse(date.as_str().unwrap_or(&now_str), &Rfc3339).unwrap();
                (self.date - date.date()).whole_days()
            }
            None => 0,
        }
    }
}
