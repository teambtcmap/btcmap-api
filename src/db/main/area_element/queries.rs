use super::{blocking_queries, schema::AreaElement};
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn insert(area_id: i64, element_id: i64, pool: &Pool) -> Result<AreaElement> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(area_id, element_id, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<AreaElement>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_updated_since(&updated_since, limit, conn))
        .await?
}

pub async fn select_by_area_id(area_id: i64, pool: &Pool) -> Result<Vec<AreaElement>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_area_id(area_id, conn))
        .await?
}

pub async fn select_by_element_id(element_id: i64, pool: &Pool) -> Result<Vec<AreaElement>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_element_id(element_id, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<AreaElement> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<AreaElement> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at.as_ref(), conn))
        .await?
}
