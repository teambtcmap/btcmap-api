use crate::{service, Result};
use deadpool_sqlite::Pool;

pub async fn run(pool: &Pool) -> Result<i64> {
    let affected_invoices = service::invoice::sync_unpaid_invoices(pool).await?;
    Ok(affected_invoices)
}
