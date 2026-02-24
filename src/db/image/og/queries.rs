use super::blocking_queries;
use super::schema::OgImage;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    element_id: i64,
    version: i64,
    image_data: Vec<u8>,
    pool: &Pool,
) -> Result<OgImage> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(element_id, version, image_data, conn))
        .await?
}

pub async fn select_by_element_id(element_id: i64, pool: &Pool) -> Result<Option<OgImage>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_element_id(element_id, conn))
        .await?
}

pub async fn delete(element_id: i64, pool: &Pool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::delete(element_id, conn))
        .await?
}
