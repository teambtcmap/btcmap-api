use super::{queries, schema::Boost};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select_all(pool: &Pool) -> Result<Vec<Boost>> {
    pool.get()
        .await?
        .interact(|conn| queries::select_all(conn))
        .await?
}
