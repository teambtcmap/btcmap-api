use crate::Result;
use serde::Serialize;

pub const NAME: &str = "paywall_get_boost_element_quote";

#[derive(Serialize)]
pub struct Res {
    pub quote_30d_sats: i64,
    pub quote_90d_sats: i64,
    pub quote_365d_sats: i64,
}

pub async fn run() -> Result<Res> {
    Ok(Res {
        quote_30d_sats: 5000,
        quote_90d_sats: 10000,
        quote_365d_sats: 30000,
    })
}
