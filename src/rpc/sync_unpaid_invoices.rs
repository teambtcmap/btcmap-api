use crate::{service, service::matrix, Result};
use deadpool_sqlite::Pool;

pub async fn run(pool: &Pool) -> Result<i64> {
    let matrix_client = matrix::try_client(pool);
    let affected_invoices = service::invoice::sync_unpaid_invoices(pool, &matrix_client).await?;
    Ok(affected_invoices)
}
