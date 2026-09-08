use geojson::JsonObject;
use rusqlite::Row;
use std::sync::OnceLock;
use time::{format_description::well_known::Rfc3339, Date, OffsetDateTime};

pub const TABLE_NAME: &str = "report";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    AreaId,
    Date,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
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
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Report> {
        |row: &_| {
            let tags: String = row.get(Columns::Tags.as_ref())?;
            let tags = serde_json::from_str(&tags).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Report {
                id: row.get(Columns::Id.as_ref())?,
                area_id: row.get(Columns::AreaId.as_ref())?,
                date: row.get(Columns::Date.as_ref())?,
                tags,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }

    pub fn total_merchants(&self) -> i64 {
        if self.tags.contains_key("total_merchants") {
            self.tags
                .get("total_merchants")
                .map(|it| it.as_i64().unwrap_or_default())
                .unwrap_or_default()
        } else {
            self.total_elements() - self.total_atms()
        }
    }

    pub fn total_elements(&self) -> i64 {
        self.tags
            .get("total_elements")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn total_exchanges(&self) -> i64 {
        if self.tags.contains_key("total_exchanges") {
            self.tags
                .get("total_exchanges")
                .map(|it| it.as_i64().unwrap_or_default())
                .unwrap_or_default()
        } else {
            self.total_atms()
        }
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

#[cfg(test)]
mod test {
    use super::{Columns, Report};

    #[test]
    fn columns_as_ref() {
        assert_eq!(Columns::Id.as_ref(), "id");
        assert_eq!(Columns::AreaId.as_ref(), "area_id");
        assert_eq!(Columns::Date.as_ref(), "date");
        assert_eq!(Columns::Tags.as_ref(), "tags");
        assert_eq!(Columns::CreatedAt.as_ref(), "created_at");
        assert_eq!(Columns::UpdatedAt.as_ref(), "updated_at");
        assert_eq!(Columns::DeletedAt.as_ref(), "deleted_at");
    }

    #[test]
    fn projection_contains_all_columns() {
        let projection = Report::projection();
        assert!(projection.contains("id"));
        assert!(projection.contains("area_id"));
        assert!(projection.contains("date"));
        assert!(projection.contains("tags"));
        assert!(projection.contains("created_at"));
        assert!(projection.contains("updated_at"));
        assert!(projection.contains("deleted_at"));
    }
}
