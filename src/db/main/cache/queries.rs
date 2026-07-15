use super::blocking_queries;
use crate::db::main::MainPool;
use crate::Result;

pub async fn select(key: String, pool: &MainPool) -> Result<Option<String>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select(&key, conn))
        .await?
}

pub async fn upsert(key: String, value: String, pool: &MainPool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::upsert(&key, &value, conn))
        .await?
}
