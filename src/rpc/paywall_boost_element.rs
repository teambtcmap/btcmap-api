use crate::{conf::Conf, element::Element, invoice, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "paywall_boost_element";

#[derive(Deserialize)]
pub struct Args {
    pub element_id: String,
    pub days: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub payment_request: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let conf = Conf::select_async(&pool).await?;
    Element::select_by_id_or_osm_id_async(&args.element_id, &pool)
        .await?
        .ok_or(format!(
            "there is no element with id = {}",
            &args.element_id,
        ))?;
    let sats = match args.days {
        30 => conf.paywall_boost_element_30d_price_sat,
        90 => conf.paywall_boost_element_90d_price_sat,
        365 => conf.paywall_boost_element_365d_price_sat,
        _ => Err("Invalid duration")?,
    };
    let invoice = invoice::service::create(
        format!("element_boost:{}:{}", args.element_id, args.days),
        sats,
        &pool,
    )
    .await?;
    Ok(Res {
        payment_request: invoice.payment_request,
    })
}
