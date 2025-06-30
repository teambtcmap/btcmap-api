use super::{
    queries,
    schema::{ElementIssue, SelectOrderedBySeverityRow},
};
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn select_by_element_id(element_id: i64, pool: &Pool) -> Result<Vec<ElementIssue>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_element_id(element_id, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<ElementIssue>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_updated_since(&updated_since, limit, conn))
        .await?
}

pub async fn select_ordered_by_severity(
    area_id: i64,
    limit: i64,
    offset: i64,
    pool: &Pool,
) -> Result<Vec<SelectOrderedBySeverityRow>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_ordered_by_severity(area_id, limit, offset, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<ElementIssue> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}

pub async fn select_count(area_id: i64, include_deleted: bool, pool: &Pool) -> Result<i64> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_count(area_id, include_deleted, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<ElementIssue> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
