use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
pub struct ElementEvent {
    pub date: String,
    pub element_id: String,
    pub element_lat: f64,
    pub element_lon: f64,
    pub element_name: String,
    pub event_type: String,
    pub user: Option<String>,
}
