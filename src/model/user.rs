use serde_json::Value;

pub struct User {
    pub id: i64,
    pub osm_json: Value,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}
