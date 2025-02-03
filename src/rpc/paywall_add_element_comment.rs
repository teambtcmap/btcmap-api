use crate::{
    conf::Conf,
    element::Element,
    element_comment::ElementComment,
    invoice::{self},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

pub const NAME: &str = "paywall_add_element_comment";

#[derive(Deserialize)]
pub struct Params {
    pub element_id: String,
    pub comment: String,
}

#[derive(Serialize)]
pub struct Res {
    pub payment_request: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<Res> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let element = Element::select_by_id_or_osm_id_async(params.element_id, &pool)
        .await?
        .ok_or("Element not found")?;
    let comment = ElementComment::insert_async(element.id, &params.comment, &pool).await?;
    ElementComment::set_deleted_at_async(comment.id, Some(OffsetDateTime::now_utc()), &pool)
        .await?;
    let invoice = invoice::service::create(
        format!("element_comment:{}:publish", comment.id),
        conf.paywall_add_element_comment_price_sat,
        &pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
    })
}
