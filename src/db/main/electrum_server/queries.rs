use super::{blocking_queries, schema::ElectrumServer};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select_all(pool: &Pool) -> Result<Vec<ElectrumServer>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all(conn))
        .await?
}

#[allow(dead_code)]
pub async fn select_by_id(id: i64, pool: &Pool) -> Result<ElectrumServer> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn insert(
    name: String,
    url: String,
    priority: i64,
    spki_pin: String,
    pool: &Pool,
) -> Result<ElectrumServer> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&name, &url, priority, &spki_pin, conn))
        .await?
}

pub async fn update(
    id: i64,
    name: Option<String>,
    url: Option<String>,
    priority: Option<i64>,
    spki_pin: Option<String>,
    pool: &Pool,
) -> Result<ElectrumServer> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::update(
                id,
                name.as_deref(),
                url.as_deref(),
                priority,
                spki_pin.as_deref(),
                conn,
            )
        })
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<time::OffsetDateTime>,
    pool: &Pool,
) -> Result<ElectrumServer> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}
