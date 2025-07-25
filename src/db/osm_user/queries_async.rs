use super::queries::{self, SelectMostActive};
use crate::{db::osm_user::schema::OsmUser, service::osm::EditingApiUser, Result};
use deadpool_sqlite::Pool;
use serde_json::Value;
use time::OffsetDateTime;

pub async fn insert(id: i64, osm_data: EditingApiUser, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::insert(id, &osm_data, conn))
        .await?
}

pub async fn set_tag(id: i64, name: String, value: Value, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::set_tag(id, &name, &value, conn))
        .await?
}

pub async fn select_most_active(
    period_start: OffsetDateTime,
    period_end: OffsetDateTime,
    limit: i64,
    pool: &Pool,
) -> Result<Vec<SelectMostActive>> {
    pool.get()
        .await?
        .interact(move |conn| {
            super::queries::select_most_active(period_start, period_end, limit, conn)
        })
        .await?
}

pub async fn select_all(limit: Option<i64>, pool: &Pool) -> Result<Vec<OsmUser>> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_all(limit, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<OsmUser>> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_updated_since(&updated_since, limit, conn))
        .await?
}

pub async fn select_by_id_or_name(id_or_name: String, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_id_or_name(&id_or_name, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_name(name: String, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_name(&name, conn))
        .await?
}

pub async fn set_osm_data(id: i64, osm_data: EditingApiUser, pool: &Pool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_osm_data(id, &osm_data, conn))
        .await?
}

#[cfg(test)]
pub async fn set_updated_at(id: i64, updated_at: OffsetDateTime, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_updated_at(id, updated_at, conn))
        .await?
}

pub async fn remove_tag(id: i64, name: String, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| queries::remove_tag(id, &name, conn))
        .await?
}
