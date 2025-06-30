use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "element_issue";

pub enum Columns {
    Id,
    ElementId,
    Code,
    Severity,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::ElementId => "element_id",
            Columns::Code => "code",
            Columns::Severity => "severity",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
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
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<ElementIssue> {
        |row| {
            Ok(ElementIssue {
                id: row.get(Columns::Id.as_str())?,
                element_id: row.get(Columns::ElementId.as_str())?,
                code: row.get(Columns::Code.as_str())?,
                severity: row.get(Columns::Severity.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
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
                element_osm_type: row.get(0)?,
                element_osm_id: row.get(1)?,
                element_name: row.get(2)?,
                issue_code: row.get(3)?,
            })
        }
    }
}
