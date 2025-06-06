use crate::{conf::Conf, element::Element, invoice, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub element_id: String,
    pub days: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub payment_request: String,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    Element::select_by_id_or_osm_id_async(&params.element_id, pool).await?;
    let sats = match params.days {
        30 => conf.paywall_boost_element_30d_price_sat,
        90 => conf.paywall_boost_element_90d_price_sat,
        365 => conf.paywall_boost_element_365d_price_sat,
        _ => Err("Invalid duration")?,
    };
    let invoice = invoice::service::create(
        format!("element_boost:{}:{}", params.element_id, params.days),
        sats,
        pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
    })
}
