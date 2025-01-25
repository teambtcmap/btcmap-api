use crate::{conf::Conf, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Serialize;
use std::sync::Arc;

pub const NAME: &str = "paywall_get_add_element_comment_quote";

#[derive(Serialize)]
pub struct Res {
    pub quote_sat: i64,
}

pub async fn run(pool: Data<Arc<Pool>>) -> Result<Res> {
    let conf = Conf::select_async(&pool).await?;
    Ok(Res {
        quote_sat: conf.paywall_add_element_comment_price_sat,
    })
}
