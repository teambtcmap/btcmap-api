use super::{blocking_queries, blocking_queries::InsertArgs, schema::PlaceReport};
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn insert(args: InsertArgs, pool: &Pool) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&args, conn))
        .await?
}

pub async fn select_open_and_not_deleted(pool: &Pool) -> Result<Vec<PlaceReport>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_open_and_not_deleted(conn))
        .await?
}

pub async fn set_ticket_url(id: i64, ticket_url: String, pool: &Pool) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_ticket_url(id, ticket_url, conn))
        .await?
}

pub async fn set_closed_at(
    id: i64,
    closed_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_closed_at(id, closed_at, conn))
        .await?
}

#[cfg(test)]
pub async fn select_by_id(id: i64, pool: &Pool) -> Result<PlaceReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}
