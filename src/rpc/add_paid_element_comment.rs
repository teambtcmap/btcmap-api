use crate::{
    element::Element,
    element_comment::ElementComment,
    invoice::{self},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

const _NAME: &str = "add_paid_element_comment";

#[derive(Deserialize)]
pub struct Args {
    pub _password: Option<String>,
    pub element_id: String,
    pub comment: String,
}

#[derive(Serialize)]
pub struct Res {
    pub payment_request: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let element = Element::select_by_id_or_osm_id_async(&args.element_id, &pool)
        .await?
        .ok_or(format!(
            "there is no element with id = {}",
            &args.element_id,
        ))?;
    let comment = ElementComment::insert_async(element.id, &args.comment, &pool).await?;
    ElementComment::set_deleted_at_async(comment.id, Some(OffsetDateTime::now_utc()), &pool)
        .await?;
    let invoice = invoice::service::create(
        format!("element_comment:{}:publish", comment.id),
        500,
        &pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
    })
}
