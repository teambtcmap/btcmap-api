use super::{blocking_queries, blocking_queries::InsertArgs, schema::PlaceReport};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(args: InsertArgs, pool: &Pool) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&args, conn))
        .await?
}

#[cfg(test)]
pub async fn select_by_id(id: i64, pool: &Pool) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}
