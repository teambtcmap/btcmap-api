use crate::{
    db::main::event::{blocking_queries, schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub use super::blocking_queries::RankedEvent;

#[allow(clippy::too_many_arguments)]
pub async fn insert(
    area_id: Option<i64>,
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    starts_at: OffsetDateTime,
    ends_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(area_id, lat, lon, &name, &website, starts_at, ends_at, conn)
        })
        .await?
}

pub async fn select_all(pool: &Pool) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all(conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    include_deleted: bool,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_updated_since(&updated_since, include_deleted, limit, conn)
        })
        .await?
}

pub async fn select_by_bbox(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    pool: &Pool,
) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_bbox(west, south, east, north, conn))
        .await?
}

pub async fn select_upcoming_by_bbox(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    pool: &Pool,
) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_upcoming_by_bbox(west, south, east, north, conn)
        })
        .await?
}

pub async fn select_by_search(
    query: String,
    location: Option<(f64, f64)>,
    row_limit: i64,
    pool: &Pool,
) -> Result<Vec<RankedEvent>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_search(&query, location, row_limit, conn))
        .await?
}

pub async fn count_by_search(query: String, pool: &Pool) -> Result<i64> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::count_by_search(&query, conn))
        .await?
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    id: i64,
    area_id: Option<Option<i64>>,
    lat: Option<f64>,
    lon: Option<f64>,
    name: Option<String>,
    website: Option<String>,
    starts_at: Option<OffsetDateTime>,
    ends_at: Option<Option<OffsetDateTime>>,
    pool: &Pool,
) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::update(
                id,
                area_id,
                lat,
                lon,
                name.as_deref(),
                website.as_deref(),
                starts_at,
                ends_at,
                conn,
            )
        })
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
