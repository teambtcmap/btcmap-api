use super::blocking_queries::{self, CommunityStats};
use super::schema::Area;
use crate::Result;
use deadpool_sqlite::Pool;
use serde_json::{Map, Value};
use time::OffsetDateTime;

pub async fn insert(tags: Map<String, Value>, pool: &Pool) -> Result<Area> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::insert(tags, conn))
        .await?
}

pub async fn select(
    updated_since: Option<OffsetDateTime>,
    include_deleted: bool,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Area>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select(updated_since, include_deleted, limit, conn))
        .await?
}

pub async fn select_by_search_query(
    search_query: impl Into<String>,
    pool: &Pool,
) -> Result<Vec<Area>> {
    let search_query = search_query.into();
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_by_search_query(search_query, conn))
        .await?
}

pub async fn select_by_id_or_alias(id_or_alias: impl Into<String>, pool: &Pool) -> Result<Area> {
    let id_or_alias = id_or_alias.into();
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_by_id_or_alias(id_or_alias, conn))
        .await?
}

pub async fn select_by_bbox(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    pool: &Pool,
) -> Result<Vec<Area>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_bbox(west, south, east, north, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_alias(alias: impl Into<String>, pool: &Pool) -> Result<Area> {
    let alias = alias.into();
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_by_alias(alias, conn))
        .await?
}

pub async fn patch_tags(area_id: i64, tags: Map<String, Value>, pool: &Pool) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::patch_tags(area_id, tags, conn))
        .await?
}

pub async fn remove_tag(area_id: i64, tag_name: impl Into<String>, pool: &Pool) -> Result<Area> {
    let tag_name = tag_name.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::remove_tag(area_id, tag_name, conn))
        .await?
}

#[cfg(test)]
pub async fn set_updated_at(id: i64, updated_at: OffsetDateTime, pool: &Pool) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_updated_at(id, &updated_at, conn))
        .await?
}

pub async fn set_bbox(
    id: i64,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    pool: &Pool,
) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_bbox(id, west, south, east, north, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}

pub async fn select_areas_count(pool: &Pool) -> Result<i64> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_areas_count(conn))
        .await?
}

pub async fn select_verified_areas_count(verified_since: &str, pool: &Pool) -> Result<i64> {
    let verified_since = verified_since.to_string();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_verified_areas_count(conn, &verified_since))
        .await?
}

pub async fn select_without_icon_square(pool: &Pool) -> Result<Vec<Area>> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_without_icon_square(conn))
        .await?
}

pub async fn select_top_communities(pool: &Pool) -> Result<Vec<CommunityStats>> {
    pool.get()
        .await?
        .interact(|conn| blocking_queries::select_top_communities(conn))
        .await?
}

pub async fn select_top_areas_by_type(pool: &Pool, area_type: &str) -> Result<Vec<CommunityStats>> {
    let area_type = area_type.to_string();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_top_areas_by_type(conn, &area_type))
        .await?
}
