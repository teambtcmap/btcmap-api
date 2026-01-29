use crate::{
    db::{self, conf::schema::Conf},
    service, Result,
};
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
    pub invoice_uuid: String,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    db::element::queries::select_by_id_or_osm_id(&params.element_id, pool).await?;
    let sats = match params.days {
        30 => conf.paywall_boost_element_30d_price_sat,
        90 => conf.paywall_boost_element_90d_price_sat,
        365 => conf.paywall_boost_element_365d_price_sat,
        _ => Err("Invalid duration")?,
    };
    let invoice = service::invoice::create(
        "lnbits",
        format!("element_boost:{}:{}", params.element_id, params.days),
        sats,
        pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
        invoice_uuid: invoice.uuid,
    })
}
