use crate::{conf::Conf, Result};
use jsonrpc_v2::Data;
use serde::Serialize;
use std::sync::Arc;

pub const NAME: &str = "paywall_get_boost_element_quote";

#[derive(Serialize)]
pub struct Res {
    pub quote_30d_sat: i64,
    pub quote_90d_sat: i64,
    pub quote_365d_sat: i64,
}

pub async fn run(conf: Data<Arc<Conf>>) -> Result<Res> {
    run_internal(&conf).await
}

pub async fn run_internal(conf: &Conf) -> Result<Res> {
    Ok(Res {
        quote_30d_sat: conf.paywall_boost_element_30d_price_sat,
        quote_90d_sat: conf.paywall_boost_element_90d_price_sat,
        quote_365d_sat: conf.paywall_boost_element_365d_price_sat,
    })
}
