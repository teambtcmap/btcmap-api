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

impl Into<RpcArea> for Area {
    fn into(self) -> RpcArea {
        RpcArea {
            id: self.id,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}
