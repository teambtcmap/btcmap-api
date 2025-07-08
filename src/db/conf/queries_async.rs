use super::{queries, schema::Conf};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select(pool: &Pool) -> Result<Conf> {
    pool.get()
        .await?
        .interact(|conn| queries::select(conn))
        .await?
}
