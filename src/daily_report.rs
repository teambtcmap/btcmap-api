use serde::Serialize;

#[derive(Serialize)]
pub struct DailyReport {
    pub date: String,
    pub total_elements: i64,
    pub up_to_date_elements: i64,
    pub outdated_elements: i64,
    pub legacy_elements: i64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
}
