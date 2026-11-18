use super::blocking_queries;
use super::schema::OsmElementOg;
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn upsert(
    osm_type: &str,
    osm_id: i64,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    pool: &Pool,
) -> Result<OsmElementOg> {
    let osm_type = osm_type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::upsert(
                &osm_type, osm_id, image_data, width, height, size_bytes, conn,
            )
        })
        .await?
}

pub async fn select_by_osm_id(
    osm_type: &str,
    osm_id: i64,
    pool: &Pool,
) -> Result<Option<OsmElementOg>> {
    let osm_type = osm_type.to_owned();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_osm_id(&osm_type, osm_id, conn))
        .await?
}

pub async fn delete_created_before(cutoff: OffsetDateTime, pool: &Pool) -> Result<usize> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::delete_created_before(cutoff, conn))
        .await?
}
