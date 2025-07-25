use crate::db;
use crate::db::element_comment::schema::ElementComment;
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
    pub element_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<ElementComment> for GetItem {
    fn from(val: ElementComment) -> Self {
        let element_id = if val.deleted_at.is_none() {
            Some(val.element_id)
        } else {
            None
        };
        let created_at = if val.deleted_at.is_none() {
            Some(val.created_at)
        } else {
            None
        };
        let comment = if val.deleted_at.is_none() {
            Some(val.comment)
        } else {
            None
        };
        GetItem {
            id: val.id,
            element_id,
            comment,
            created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<ElementComment> for Json<GetItem> {
    fn from(val: ElementComment) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let element_comments = db::element_comment::queries_async::select_updated_since(
        args.updated_since,
        true,
        Some(args.limit),
        &pool,
    )
    .await?;
    req.extensions_mut()
        .insert(RequestExtension::new(element_comments.len()));
    Ok(Json(
        element_comments.into_iter().map(|it| it.into()).collect(),
    ))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    db::element_comment::queries_async::select_by_id(*id, &pool)
        .await
        .map(Into::into)
}
