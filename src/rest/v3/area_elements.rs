use crate::{
    db::{self, area_element::schema::AreaElement},
    log::RequestExtension,
    Result,
};
use actix_web::{
    get,
    web::{Data, Json, Path, Query},
    HttpMessage, HttpRequest,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
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

impl From<AreaElement> for GetItem {
    fn from(val: AreaElement) -> Self {
        let area_id = if val.deleted_at.is_none() {
            Some(val.area_id)
        } else {
            None
        };
        let element_id = if val.deleted_at.is_none() {
            Some(val.element_id)
        } else {
            None
        };
        GetItem {
            id: val.id,
            area_id,
            element_id,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<AreaElement> for Json<GetItem> {
    fn from(val: AreaElement) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>> {
    let area_elements = db::area_element::queries::select_updated_since(
        args.updated_since,
        Some(args.limit),
        &pool,
    )
    .await?;
    req.extensions_mut()
        .insert(RequestExtension::new(area_elements.len()));
    Ok(Json(
        area_elements.into_iter().map(|it| it.into()).collect(),
    ))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>> {
    db::area_element::queries::select_by_id(*id, &pool)
        .await
        .map(|it| it.into())
}
