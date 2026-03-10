use std::sync::OnceLock;

pub const TABLE_NAME: &str = "sync";

pub enum Columns {
    Id,
    StartedAt,
    FinishedAt,
    DurationS,
    OverpassResponseTimeS,
    ElementsAffected,
    ElementsCreated,
    ElementsUpdated,
    ElementsDeleted,
    FailedAt,
    FailReason,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::StartedAt => "started_at",
            Columns::FinishedAt => "finished_at",
            Columns::DurationS => "duration_s",
            Columns::OverpassResponseTimeS => "overpass_response_time_s",
            Columns::ElementsAffected => "elements_affected",
            Columns::ElementsCreated => "elements_created",
            Columns::ElementsUpdated => "elements_updated",
            Columns::ElementsDeleted => "elements_deleted",
            Columns::FailedAt => "failed_at",
            Columns::FailReason => "fail_reason",
        }
    }
}

#[allow(dead_code)]
pub struct Sync {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_s: Option<f64>,
    pub overpass_response_time_s: Option<f64>,
    pub elements_affected: i64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
    pub failed_at: Option<String>,
    pub fail_reason: Option<String>,
}

#[allow(dead_code)]
impl Sync {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::StartedAt,
                Columns::FinishedAt,
                Columns::DurationS,
                Columns::OverpassResponseTimeS,
                Columns::ElementsAffected,
                Columns::ElementsCreated,
                Columns::ElementsUpdated,
                Columns::ElementsDeleted,
                Columns::FailedAt,
                Columns::FailReason,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&rusqlite::Row) -> rusqlite::Result<Self> {
        |row: &rusqlite::Row| -> rusqlite::Result<Self> {
            Ok(Sync {
                id: row.get(Columns::Id.as_str())?,
                started_at: row.get(Columns::StartedAt.as_str())?,
                finished_at: row.get(Columns::FinishedAt.as_str())?,
                duration_s: row.get(Columns::DurationS.as_str())?,
                overpass_response_time_s: row.get(Columns::OverpassResponseTimeS.as_str())?,
                elements_affected: row.get(Columns::ElementsAffected.as_str())?,
                elements_created: row.get(Columns::ElementsCreated.as_str())?,
                elements_updated: row.get(Columns::ElementsUpdated.as_str())?,
                elements_deleted: row.get(Columns::ElementsDeleted.as_str())?,
                failed_at: row.get(Columns::FailedAt.as_str())?,
                fail_reason: row.get(Columns::FailReason.as_str())?,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Columns;

    #[test]
    fn columns_as_str() {
        assert_eq!(Columns::Id.as_str(), "id");
        assert_eq!(Columns::StartedAt.as_str(), "started_at");
        assert_eq!(Columns::FinishedAt.as_str(), "finished_at");
        assert_eq!(Columns::DurationS.as_str(), "duration_s");
        assert_eq!(
            Columns::OverpassResponseTimeS.as_str(),
            "overpass_response_time_s"
        );
        assert_eq!(Columns::ElementsAffected.as_str(), "elements_affected");
        assert_eq!(Columns::ElementsCreated.as_str(), "elements_created");
        assert_eq!(Columns::ElementsUpdated.as_str(), "elements_updated");
        assert_eq!(Columns::ElementsDeleted.as_str(), "elements_deleted");
    }

    #[test]
    fn sync_projection() {
        let projection = super::Sync::projection();
        assert!(projection.contains("id"));
        assert!(projection.contains("started_at"));
        assert!(projection.contains("finished_at"));
        assert!(projection.contains("duration_s"));
        assert!(projection.contains("overpass_response_time_s"));
        assert!(projection.contains("elements_affected"));
        assert!(projection.contains("elements_created"));
        assert!(projection.contains("elements_updated"));
        assert!(projection.contains("elements_deleted"));
    }
}
