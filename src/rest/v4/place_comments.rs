use std::i64;

use crate::db;
use crate::db::conf::schema::Conf;
use crate::db::element_comment::schema::ElementComment;
use crate::log::RequestExtension;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::post;
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
pub struct Args {
    #[serde(default = "default_updated_since")]
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default = "default_include_deleted")]
    include_deleted: bool,
}

const fn default_updated_since() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

const fn default_limit() -> i64 {
    i64::MAX
}

const fn default_include_deleted() -> bool {
    false
}

#[derive(Serialize)]
pub struct Item {
    pub id: i64,
    pub place_id: i64,
    pub text: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<ElementComment> for Item {
    fn from(val: ElementComment) -> Self {
        Item {
            id: val.id,
            place_id: val.element_id,
            text: val.comment,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<ElementComment> for Json<Item> {
    fn from(val: ElementComment) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<Args>,
    pool: Data<Pool>,
) -> Result<Json<Vec<Item>>, Error> {
    let items = db::element_comment::queries::select_updated_since(
        args.updated_since,
        args.include_deleted,
        Some(args.limit),
        &pool,
    )
    .await?;
    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));
    Ok(Json(items.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<Item>, Error> {
    db::element_comment::queries::select_by_id(*id, &pool)
        .await
        .map(Into::into)
}

#[derive(Serialize)]
pub struct Quote {
    pub quote_sat: i64,
}

#[get("/quote")]
pub async fn get_quote(conf: Data<Conf>) -> RestResult<Quote> {
    Ok(Json(Quote {
        quote_sat: conf.paywall_add_element_comment_price_sat,
    }))
}

#[derive(Deserialize)]
pub struct PostArgs {
    pub place_id: String,
    pub comment: String,
}

#[derive(Serialize)]
pub struct PostResponse {
    pub invoice_id: String,
    pub invoice: String,
}

#[post("")]
pub async fn post(
    args: Json<PostArgs>,
    conf: Data<Conf>,
    pool: Data<Pool>,
) -> RestResult<PostResponse> {
    let element = db::element::queries::select_by_id_or_osm_id(&args.place_id, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    let comment = db::element_comment::queries::insert(element.id, &args.comment, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    db::element_comment::queries::set_deleted_at(
        comment.id,
        Some(OffsetDateTime::now_utc()),
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    let invoice = service::invoice::create(
        "lnd",
        format!("element_comment:{}:publish", comment.id),
        conf.paywall_add_element_comment_price_sat,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    Ok(Json(PostResponse {
        invoice_id: invoice.uuid,
        invoice: invoice.payment_request,
    }))
}
