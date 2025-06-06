use super::queries;
use crate::element::Element;
use crate::osm::overpass::OverpassElement;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(overpass_data: OverpassElement, pool: &Pool) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::insert(&overpass_data, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_by_id(id, conn))
        .await?
}
