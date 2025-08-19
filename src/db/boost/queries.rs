use super::{blocking_queries, schema::Boost};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    admin_id: i64,
    element_id: i64,
    duration_days: i64,
    pool: &Pool,
) -> Result<Boost> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(admin_id, element_id, duration_days, conn))
        .await?
}

pub async fn select_all(pool: &Pool) -> Result<Vec<Boost>> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_all(conn))
        .await?
}
