use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Element {
    pub id: String,
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}
