use super::queries;
use crate::element::Element;
use crate::osm::overpass::OverpassElement;
use crate::Result;
use deadpool_sqlite::Pool;
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

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}
