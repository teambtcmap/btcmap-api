use super::model::ElementIssue;
use crate::log::RequestExtension;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use time::macros::datetime;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetItem {
    pub id: i64,
    pub element_id: i64,
    pub code: String,
    pub severity: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

impl From<ElementIssue> for GetItem {
    fn from(val: ElementIssue) -> Self {
        GetItem {
            id: val.id,
            element_id: val.element_id,
            code: val.code,
            severity: val.severity,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<ElementIssue> for Json<GetItem> {
    fn from(val: ElementIssue) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let items = pool
        .get()
        .await?
        .interact(move |conn| {
            ElementIssue::select_updated_since(
                &args
                    .updated_since
                    .unwrap_or(datetime!(2000-01-01 00:00 UTC)),
                args.limit,
                conn,
            )
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));
    Ok(Json(items.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    pool.get()
        .await?
        .interact(move |conn| ElementIssue::select_by_id(*id, conn))
        .await?
        .map(|it| it.into())
}
