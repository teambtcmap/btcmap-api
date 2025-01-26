use crate::{
    admin,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;

const NAME: &str = "sync_unpaid_invoices";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Vec<Invoice>> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    let affected_invoices = invoice::service::sync_unpaid_invoices(&pool).await?;
    Ok(affected_invoices)
}
