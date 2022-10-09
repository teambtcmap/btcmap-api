use serde::Serialize;

#[derive(Serialize)]
pub struct DailyReport {
    pub area_id: String,
    pub date: String,
    pub total_elements: i64,
    pub total_elements_onchain: i64,
    pub total_elements_lightning: i64,
    pub total_elements_lightning_contactless: i64,
    pub up_to_date_elements: i64,
    pub outdated_elements: i64,
    pub legacy_elements: i64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}
