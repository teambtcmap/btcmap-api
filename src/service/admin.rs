use crate::{db, Result};
use deadpool_sqlite::Pool;
use tracing::warn;

pub async fn upgrade_plaintext_passwords(pool: &Pool) -> Result<i64> {
    let admins = db::admin::queries_async::select_all(pool).await?;
    warn!("Loaded {} admin users", admins.len());
    Ok(0)
}
