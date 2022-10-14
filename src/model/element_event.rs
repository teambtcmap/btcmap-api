use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct ElementEvent {
    pub id: i64,
    pub date: String,
    pub element_id: String,
    pub element_lat: f64,
    pub element_lon: f64,
    pub element_name: String,
    pub event_type: String,
    pub user_id: i64,
    pub user: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}
