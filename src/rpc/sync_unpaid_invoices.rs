use crate::{
    admin,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

pub const NAME: &str = "sync_unpaid_invoices";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Invoice>> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let affected_invoices = invoice::service::sync_unpaid_invoices(&pool).await?;
    Ok(affected_invoices)
}
