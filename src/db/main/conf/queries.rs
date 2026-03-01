use super::{blocking_queries, schema::Conf};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select(pool: &Pool) -> Result<Conf> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select(conn))
        .await?
}
