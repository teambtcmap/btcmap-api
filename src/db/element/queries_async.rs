use super::queries;
use crate::element::Element;
use crate::osm::overpass::OverpassElement;
use crate::Result;
use deadpool_sqlite::Pool;
use serde_json::Value;
use time::OffsetDateTime;

pub async fn insert(overpass_data: OverpassElement, pool: &Pool) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::insert(&overpass_data, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    include_deleted: bool,
    pool: &Pool,
) -> Result<Vec<Element>> {
    pool.get()
        .await?
        .interact(move |conn| {
            queries::select_updated_since(updated_since, limit, include_deleted, conn)
        })
        .await?
}

pub async fn select_by_search_query(
    search_query: impl Into<String>,
    pool: &Pool,
) -> Result<Vec<Element>> {
    let search_query = search_query.into();
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_search_query(search_query, conn))
        .await?
}

pub async fn select_by_id_or_osm_id(id: impl Into<String>, pool: &Pool) -> Result<Element> {
    let id = id.into();
    pool.get()
        .await?
        .interact(|conn| queries::select_by_id_or_osm_id(id, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}

pub async fn set_overpass_data(
    id: i64,
    overpass_data: OverpassElement,
    pool: &Pool,
) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_overpass_data(id, &overpass_data, conn))
        .await?
}

pub async fn set_tag(
    id: i64,
    name: impl Into<String>,
    value: &Value,
    pool: &Pool,
) -> Result<Element> {
    let name = name.into();
    let value = value.clone();
    pool.get()
        .await?
        .interact(move |conn| queries::set_tag(id, &name, &value, conn))
        .await?
}

pub async fn remove_tag(
    element_id: i64,
    tag_name: impl Into<String>,
    pool: &Pool,
) -> Result<Element> {
    let tag_name = tag_name.into();
    pool.get()
        .await?
        .interact(move |conn| queries::remove_tag(element_id, &tag_name, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
