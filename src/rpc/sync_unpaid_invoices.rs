use crate::{service, Result};
use deadpool_sqlite::Pool;
use matrix_sdk::Client;

pub async fn run(pool: &Pool, matrix_client: Option<Client>) -> Result<i64> {
    let affected_invoices = service::invoice::sync_unpaid_invoices(pool, &matrix_client).await?;
    Ok(affected_invoices)
}
