use crate::{
    db::{self, conf::schema::Conf},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub element_id: String,
    pub comment: String,
}

#[derive(Serialize)]
pub struct Res {
    pub payment_request: String,
    pub invoice_uuid: String,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let element =
        db::element::queries_async::select_by_id_or_osm_id(params.element_id, pool).await?;
    let comment =
        db::element_comment::queries_async::insert(element.id, &params.comment, pool).await?;
    db::element_comment::queries_async::set_deleted_at(
        comment.id,
        Some(OffsetDateTime::now_utc()),
        pool,
    )
    .await?;
    let invoice = service::invoice::create(
        format!("element_comment:{}:publish", comment.id),
        conf.paywall_add_element_comment_price_sat,
        pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
        invoice_uuid: invoice.uuid,
    })
}
