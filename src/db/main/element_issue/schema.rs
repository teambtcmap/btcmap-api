use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "element_issue";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    ElementId,
    Code,
    Severity,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ElementIssue {
    pub id: i64,
    pub element_id: i64,
    pub code: String,
    pub severity: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl ElementIssue {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::ElementId,
                Columns::Code,
                Columns::Severity,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<ElementIssue> {
        |row| {
            Ok(ElementIssue {
                id: row.get(Columns::Id.as_ref())?,
                element_id: row.get(Columns::ElementId.as_ref())?,
                code: row.get(Columns::Code.as_ref())?,
                severity: row.get(Columns::Severity.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}

pub struct SelectOrderedBySeverityRow {
    pub element_osm_type: String,
    pub element_osm_id: i64,
    pub element_name: Option<String>,
    pub issue_code: String,
}

impl SelectOrderedBySeverityRow {
    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<SelectOrderedBySeverityRow> {
        |row| {
            Ok(SelectOrderedBySeverityRow {
                element_osm_type: row.get("element_osm_type")?,
                element_osm_id: row.get("element_osm_id")?,
                element_name: row.get("element_name")?,
                issue_code: row.get("issue_code")?,
            })
        }
    }
}
