use crate::{
    admin,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "sync_unpaid_invoices";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<Invoice>> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Invoice>> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let affected_invoices = invoice::service::sync_unpaid_invoices(&pool).await?;
    Ok(affected_invoices)
}
