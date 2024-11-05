use crate::area::Area;
use serde::Serialize;
use serde_json::{Map, Value};
use time::OffsetDateTime;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct RpcArea {
    pub id: i64,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for RpcArea {
    fn from(val: Area) -> Self {
        RpcArea {
            id: val.id,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}
