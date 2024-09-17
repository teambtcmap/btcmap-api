use super::model::AreaElement;
use crate::{Error, Result};
use actix_web::{
    get,
    web::{Data, Json, Path, Query},
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    limit: i64,
}

#[derive(Serialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: Option<i64>,
    pub element_id: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for AreaElement {
    fn into(self) -> GetItem {
        let area_id = if self.deleted_at.is_none() {
            Some(self.area_id)
        } else {
            None
        };
        let element_id = if self.deleted_at.is_none() {
            Some(self.element_id)
        } else {
            None
        };
        GetItem {
            id: self.id,
            area_id,
            element_id,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for AreaElement {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(args: Query<GetArgs>, pool: Data<Arc<Pool>>) -> Result<Json<Vec<GetItem>>> {
    let areas = pool
        .get()
        .await?
        .interact(move |conn| {
            AreaElement::select_updated_since(&args.updated_since, Some(args.limit), conn)
        })
        .await??;
    Ok(Json(areas.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Arc<Pool>>) -> Result<Json<GetItem>> {
    let id_clone = id.clone();
    pool.get()
        .await?
        .interact(move |conn| AreaElement::select_by_id(id_clone, conn))
        .await??
        .ok_or(Error::HttpNotFound(format!(
            "Area-element mapping with id {id} doesn't exist"
        )))
        .map(|it| it.into())
}
