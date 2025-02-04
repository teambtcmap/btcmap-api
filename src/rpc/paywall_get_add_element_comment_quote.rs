use crate::{conf::Conf, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub quote_sat: i64,
}

pub async fn run_internal(conf: &Conf) -> Result<Res> {
    Ok(Res {
        quote_sat: conf.paywall_add_element_comment_price_sat,
    })
}
