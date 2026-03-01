use super::{blocking_queries, schema::Ban};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(ip: String, reason: String, duration_days: i64, pool: &Pool) -> Result<Ban> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&ip, &reason, duration_days, conn))
        .await?
}

pub async fn select_by_ip(ip: String, pool: &Pool) -> Result<Option<Ban>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_ip(&ip, conn))
        .await?
}
