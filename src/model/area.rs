use std::collections::HashMap;

use serde_json::Value;
use time::OffsetDateTime;

#[derive(PartialEq, Debug)]
pub struct Area {
    pub id: i64,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}
