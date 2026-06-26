use super::{blocking_queries, schema::ImportOrigin};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select_by_name(name: String, pool: &Pool) -> Result<Option<ImportOrigin>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_name(&name, conn))
        .await?
}

pub async fn select_all(pool: &Pool) -> Result<Vec<ImportOrigin>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all(conn))
        .await?
}
