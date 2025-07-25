use super::{queries, schema::Report};
use crate::Result;
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use time::{Date, OffsetDateTime};

pub async fn insert(area_id: i64, date: Date, tags: JsonObject, pool: &Pool) -> Result<Report> {
    pool.get()
        .await?
        .interact(move |conn| queries::insert(area_id, date, &tags, conn))
        .await?
}

pub async fn select_all(
    sort_order: Option<String>,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Report>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_all(sort_order, limit, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Report>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_updated_since(updated_since, limit, conn))
        .await?
}

pub async fn select_by_date(date: Date, limit: Option<i64>, pool: &Pool) -> Result<Vec<Report>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_date(date, limit, conn))
        .await?
}

pub async fn select_latest_by_area_id(area_id: i64, pool: &Pool) -> Result<Report> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_latest_by_area_id(area_id, conn))
        .await?
}

pub async fn select_by_area_id(
    area_id: i64,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Report>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_area_id(area_id, limit, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Report> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}

pub async fn patch_tags(id: i64, tags: JsonObject, pool: &Pool) -> Result<Report> {
    pool.get()
        .await?
        .interact(move |conn| queries::patch_tags(id, &tags, conn))
        .await?
}

#[cfg(test)]
pub async fn set_updated_at(
    id: i64,
    updated_at: time::OffsetDateTime,
    pool: &Pool,
) -> Result<Report> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_updated_at(id, updated_at, conn))
        .await?
}
