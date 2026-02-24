use super::super::LogPool;
use super::blocking_queries;
use super::blocking_queries::InsertArgs;
use crate::Result;

pub async fn insert(args: InsertArgs, pool: &LogPool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(args, conn))
        .await?
}
