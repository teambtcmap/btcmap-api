use super::blocking_queries;
use super::schema::OgImage;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    element_id: i64,
    version: i64,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    pool: &Pool,
) -> Result<OgImage> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(element_id, version, image_data, width, height, size_bytes, conn)
        })
        .await?
}

pub async fn select_by_element_id(element_id: i64, pool: &Pool) -> Result<Option<OgImage>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_element_id(element_id, conn))
        .await?
}

pub async fn select_all_with_zero_metadata(pool: &Pool) -> Result<Vec<OgImage>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all_with_zero_metadata(conn))
        .await?
}

pub async fn update_metadata(
    element_id: i64,
    width: i64,
    height: i64,
    size_bytes: i64,
    pool: &Pool,
) -> Result<usize> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::update_metadata(element_id, width, height, size_bytes, conn)
        })
        .await?
}

pub async fn delete(element_id: i64, pool: &Pool) -> Result<usize> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::delete(element_id, conn))
        .await?
}
