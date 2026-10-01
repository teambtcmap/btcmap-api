use super::blocking_queries;
use super::schema::{PlaceImage, PlaceImageMeta};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    place_id: i64,
    r#type: &str,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    pool: &Pool,
) -> Result<PlaceImage> {
    let r#type = r#type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                place_id, &r#type, image_data, width, height, size_bytes, conn,
            )
        })
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<PlaceImage> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_place_id(place_id: i64, pool: &Pool) -> Result<Vec<PlaceImageMeta>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_place_id(place_id, conn))
        .await?
}

pub async fn select_by_place_id_and_type(
    place_id: i64,
    r#type: &str,
    pool: &Pool,
) -> Result<Vec<PlaceImageMeta>> {
    let r#type = r#type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_by_place_id_and_type(place_id, &r#type, conn)
        })
        .await?
}

#[allow(dead_code)]
pub async fn delete(id: i64, pool: &Pool) -> Result<usize> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::delete(id, conn))
        .await?
}
