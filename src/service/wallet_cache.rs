use crate::db::main::cache::queries as cache_queries;
use crate::db::main::MainPool;
use crate::service::wallet::{run as fetch_wallet, Res as WalletRes};
use crate::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::warn;

pub(crate) const CACHE_KEY: &str = "wallet_snapshot";

pub const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(with = "time::serde::rfc3339")]
    pub fetched_at: OffsetDateTime,
    #[serde(flatten)]
    pub res: WalletRes,
}

pub fn init(pool: &MainPool) {
    let pool = pool.clone();
    tokio::spawn(async move {
        refresh(&pool).await;
        loop {
            tokio::time::sleep(REFRESH_INTERVAL).await;
            refresh(&pool).await;
        }
    });
}

async fn refresh(pool: &MainPool) {
    match fetch_wallet(pool).await {
        Ok(res) => {
            let snapshot = Snapshot {
                fetched_at: OffsetDateTime::now_utc(),
                res,
            };
            if let Err(err) = store(&snapshot, pool).await {
                warn!(%err, "wallet snapshot refresher: failed to persist");
            }
        }
        Err(err) => {
            warn!(%err, "wallet snapshot refresher: fetch failed; cache row untouched");
        }
    }
}

pub async fn get_or_fetch(pool: &MainPool) -> Result<Snapshot> {
    if let Some(snapshot) = load(pool).await? {
        return Ok(snapshot);
    }
    match fetch_wallet(pool).await {
        Ok(res) => {
            let snapshot = Snapshot {
                fetched_at: OffsetDateTime::now_utc(),
                res,
            };
            if let Err(err) = store(&snapshot, pool).await {
                warn!(%err, "failed to persist wallet snapshot to cache table");
            }
            Ok(snapshot)
        }
        Err(err) => Err(err),
    }
}

async fn load(pool: &MainPool) -> Result<Option<Snapshot>> {
    let raw = cache_queries::select(CACHE_KEY.to_string(), pool).await?;
    match raw {
        None => Ok(None),
        Some(s) => match serde_json::from_str::<Snapshot>(&s) {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(err) => {
                warn!(%err, "failed to deserialize cached wallet snapshot; ignoring");
                Ok(None)
            }
        },
    }
}

async fn store(snapshot: &Snapshot, pool: &MainPool) -> Result<()> {
    let value = serde_json::to_string(snapshot).map_err(|err| {
        crate::Error::Other(format!("failed to serialize wallet snapshot: {}", err))
    })?;
    cache_queries::upsert(CACHE_KEY.to_string(), value, pool).await
}
