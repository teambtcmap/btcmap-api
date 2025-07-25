use super::{queries, schema::ElementComment};
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn insert(
    element_id: i64,
    comment: impl Into<String>,
    pool: &Pool,
) -> Result<ElementComment> {
    let comment = comment.into();
    pool.get()
        .await?
        .interact(move |conn| queries::insert(element_id, comment, conn))
        .await?
}

pub async fn select_latest(limit: i64, pool: &Pool) -> Result<Vec<ElementComment>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_latest(limit, conn))
        .await?
}

pub async fn select_created_between(
    period_start: OffsetDateTime,
    period_end: OffsetDateTime,
    pool: &Pool,
) -> Result<Vec<ElementComment>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_created_between(&period_start, &period_end, conn))
        .await?
}

pub async fn select_by_element_id(
    element_id: i64,
    include_deleted: bool,
    limit: i64,
    pool: &Pool,
) -> Result<Vec<ElementComment>> {
    pool.get()
        .await?
        .interact(move |conn| {
            queries::select_by_element_id(element_id, include_deleted, limit, conn)
        })
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<ElementComment> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<ElementComment> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
