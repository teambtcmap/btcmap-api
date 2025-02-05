use crate::{conf::Conf, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub quote_30d_sat: i64,
    pub quote_90d_sat: i64,
    pub quote_365d_sat: i64,
}

pub async fn run(conf: &Conf) -> Result<Res> {
    Ok(Res {
        quote_30d_sat: conf.paywall_boost_element_30d_price_sat,
        quote_90d_sat: conf.paywall_boost_element_90d_price_sat,
        quote_365d_sat: conf.paywall_boost_element_365d_price_sat,
    })
}
