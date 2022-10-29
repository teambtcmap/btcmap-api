use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Report {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}
