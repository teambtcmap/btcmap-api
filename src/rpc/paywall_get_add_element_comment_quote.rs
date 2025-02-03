use crate::{conf::Conf, Result};
use jsonrpc_v2::Data;
use serde::Serialize;
use std::sync::Arc;

pub const NAME: &str = "paywall_get_add_element_comment_quote";

#[derive(Serialize)]
pub struct Res {
    pub quote_sat: i64,
}

pub async fn run(conf: Data<Arc<Conf>>) -> Result<Res> {
    run_internal(&conf).await
}

pub async fn run_internal(conf: &Conf) -> Result<Res> {
    Ok(Res {
        quote_sat: conf.paywall_add_element_comment_price_sat,
    })
}
