use crate::{
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;

pub async fn run_internal(pool: &Pool) -> Result<Vec<Invoice>> {
    let affected_invoices = invoice::service::sync_unpaid_invoices(&pool).await?;
    Ok(affected_invoices)
}
