use super::queries::{OsmUser, SelectMostActive};
use crate::{osm::api::EditingApiUser, Result};
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

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<OsmUser> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_id(id, conn))
        .await?
}

pub async fn set_osm_data(id: i64, osm_data: EditingApiUser, pool: &Pool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::set_osm_data(id, &osm_data, conn))
        .await?
}
