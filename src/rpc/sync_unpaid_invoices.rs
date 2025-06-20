use crate::{
    invoice::{self},
    Result,
};
use deadpool_sqlite::Pool;

pub async fn run(pool: &Pool) -> Result<i64> {
    let affected_invoices = invoice::service::sync_unpaid_invoices(pool).await?;
    Ok(affected_invoices)
}
