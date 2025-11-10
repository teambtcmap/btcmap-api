use crate::{
    db::event::{blocking_queries, schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn insert(
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    starts_at: Option<OffsetDateTime>,
    ends_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(lat, lon, &name, &website, starts_at, ends_at, conn)
        })
        .await?
}

pub async fn select_all(pool: &Pool) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all(conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
