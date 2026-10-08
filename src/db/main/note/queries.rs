use super::{blocking_queries, schema::Note};
use crate::Result;
use deadpool_sqlite::Pool;
use time::OffsetDateTime;

pub async fn insert(
    user_id: i64,
    lat: f64,
    lon: f64,
    text: impl Into<String>,
    public: bool,
    icon: impl Into<String>,
    pool: &Pool,
) -> Result<Note> {
    let text = text.into();
    let icon = icon.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(user_id, lat, lon, text, public, icon, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Note> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_user_id(
    user_id: i64,
    updated_since: OffsetDateTime,
    include_deleted: bool,
    limit: i64,
    pool: &Pool,
) -> Result<Vec<Note>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_by_user_id(
                user_id,
                &updated_since,
                include_deleted,
                limit,
                conn,
            )
        })
        .await?
}

pub async fn select_public_in_bbox(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    limit: i64,
    pool: &Pool,
) -> Result<Vec<Note>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_public_in_bbox(min_lat, max_lat, min_lon, max_lon, limit, conn)
        })
        .await?
}

pub async fn update(
    id: i64,
    text: impl Into<String>,
    public: bool,
    icon: impl Into<String>,
    pool: &Pool,
) -> Result<Note> {
    let text = text.into();
    let icon = icon.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::update(id, text, public, icon, conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<Note> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
