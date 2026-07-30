use super::{
    blocking_queries,
    schema::{CachedTx, Wallet},
};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn select_all(pool: &Pool) -> Result<Vec<Wallet>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_all(conn))
        .await?
}

#[allow(dead_code)]
pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Wallet> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn insert(name: String, xpub: String, pool: &Pool) -> Result<Wallet> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&name, &xpub, conn))
        .await?
}

pub async fn update(
    id: i64,
    name: Option<String>,
    xpub: Option<String>,
    pool: &Pool,
) -> Result<Wallet> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::update(id, name.as_deref(), xpub.as_deref(), conn))
        .await?
}

pub async fn set_deleted_at(
    id: i64,
    deleted_at: Option<time::OffsetDateTime>,
    pool: &Pool,
) -> Result<Wallet> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_deleted_at(id, deleted_at, conn))
        .await?
}

pub async fn set_cached_snapshot(
    id: i64,
    balance_sats: i64,
    tx: Vec<CachedTx>,
    cached_at: time::OffsetDateTime,
    pool: &Pool,
) -> Result<Wallet> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::set_cached_snapshot(id, balance_sats, &tx, cached_at, conn)
        })
        .await?
}
