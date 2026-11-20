use crate::service::sync_lock::SyncLock;
use crate::{service, service::matrix, Result};
use deadpool_sqlite::Pool;
use std::sync::atomic::AtomicBool;

static SYNC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub async fn run(pool: &Pool) -> Result<i64> {
    let _guard = SyncLock::try_acquire(&SYNC_IN_PROGRESS)
        .ok_or("Sync of unpaid invoices is already in progress")?;
    let matrix_client = matrix::try_client(pool);
    let affected_invoices = service::invoice::sync_unpaid_invoices(pool, &matrix_client).await?;
    Ok(affected_invoices)
}
