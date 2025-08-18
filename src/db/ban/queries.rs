use super::{blocking_queries, schema::Ban};
use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::ToSql;

pub async fn select_by_ip(
    ip: impl AsRef<str> + ToSql + Send + 'static,
    pool: &Pool,
) -> Result<Option<Ban>> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_by_ip(ip, conn))
        .await?
}
