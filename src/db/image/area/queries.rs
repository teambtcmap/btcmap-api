use super::blocking_queries;
use super::schema::AreaImage;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    area_id: i64,
    r#type: &str,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    pool: &Pool,
) -> Result<AreaImage> {
    let r#type = r#type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                area_id, &r#type, image_data, width, height, size_bytes, conn,
            )
        })
        .await?
}

pub async fn select_by_area_id_and_type(
    area_id: i64,
    r#type: &str,
    pool: &Pool,
) -> Result<Option<AreaImage>> {
    let r#type = r#type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_area_id_and_type(area_id, &r#type, conn))
        .await?
}

pub async fn delete(id: i64, pool: &Pool) -> Result<usize> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::delete(id, conn))
        .await?
}
