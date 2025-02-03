use crate::{conf::Conf, element::Element, invoice, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "paywall_boost_element";

#[derive(Deserialize)]
pub struct Params {
    pub element_id: String,
    pub days: i64,
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
    Element::select_by_id_or_osm_id_async(&params.element_id, &pool)
        .await?
        .ok_or("Element not found")?;
    let sats = match params.days {
        30 => conf.paywall_boost_element_30d_price_sat,
        90 => conf.paywall_boost_element_90d_price_sat,
        365 => conf.paywall_boost_element_365d_price_sat,
        _ => Err("Invalid duration")?,
    };
    let invoice = invoice::service::create(
        format!("element_boost:{}:{}", params.element_id, params.days),
        sats,
        &pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
    })
}
