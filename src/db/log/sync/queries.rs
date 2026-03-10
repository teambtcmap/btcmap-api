use super::super::LogPool;
use super::blocking_queries;
use crate::Result;

pub async fn insert(pool: &LogPool) -> Result<i64> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(conn))
        .await?
}

pub async fn update_completed(args: blocking_queries::UpdateArgs, pool: &LogPool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::update(args, conn))
        .await?
}

pub async fn update_failed(args: blocking_queries::UpdateFailedArgs, pool: &LogPool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::update_failed(args, conn))
        .await?
}
