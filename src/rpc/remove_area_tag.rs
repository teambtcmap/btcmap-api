use crate::{
    db::area::schema::Area,
    service::{self},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub tag: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: JsonObject,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for Res {
    fn from(val: Area) -> Self {
        Res {
            id: val.id,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    service::area::remove_tag_async(params.id, &params.tag, pool)
        .await
        .map(Into::into)
}
