use crate::db::main::cache::queries as cache_queries;
use crate::db::main::conf::queries as conf_queries;
use crate::db::main::conf::schema::Conf;
use crate::db::main::MainPool;
use crate::service::wallet::{aggregate, Res as WalletRes};
use crate::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub(crate) const CACHE_KEY: &str = "wallet_snapshot";

pub const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// Upper bound on how long the cache task or dashboard RPC will wait for a
/// single wallet refresh to complete. The actual blocking work runs on a
/// detached `std::thread` that the actix tokio runtime does NOT know about,
/// so the process can always exit even if the worker is stuck — this
/// timeout just unblocks the *awaiter* (the cache loop or the RPC handler)
/// so it can move on or surface an error to the caller.
pub const REFRESH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(with = "time::serde::rfc3339")]
    pub fetched_at: OffsetDateTime,
    #[serde(flatten)]
    pub res: WalletRes,
}

pub fn init(pool: &MainPool, shutdown: CancellationToken) {
    let pool = pool.clone();
    tokio::spawn(async move {
        info!(
            refresh_interval_secs = REFRESH_INTERVAL.as_secs(),
            refresh_timeout_secs = REFRESH_TIMEOUT.as_secs(),
            "wallet snapshot refresher: started"
        );
        if run_refresh(&pool, &shutdown).await {
            return;
        }
        loop {
            info!(
                refresh_interval_secs = REFRESH_INTERVAL.as_secs(),
                "wallet snapshot refresher: waiting for next refresh"
            );
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = tokio::time::sleep(REFRESH_INTERVAL) => {}
            }
            if run_refresh(&pool, &shutdown).await {
                break;
            }
        }
        info!("wallet snapshot refresher: stopped");
    });
}

/// Runs one refresh cycle. Returns `true` if shutdown was requested (the
/// caller should stop scheduling further refreshes).
async fn run_refresh(pool: &MainPool, shutdown: &CancellationToken) -> bool {
    let started_at = std::time::Instant::now();
    info!("wallet snapshot refresher: refresh started");
    let conf = match conf_queries::select(pool).await {
        Ok(c) => c,
        Err(err) => {
            warn!(%err, "wallet snapshot refresher: failed to load conf");
            return false;
        }
    };
    let rx = spawn_blocking_refresh(conf);
    tokio::select! {
        _ = shutdown.cancelled() => true,
        result = tokio::time::timeout(REFRESH_TIMEOUT, rx) => {
            match result {
                Ok(Ok(Ok(res))) => {
                    info!(
                        elapsed = ?started_at.elapsed(),
                        "wallet snapshot refresher: fetch succeeded"
                    );
                    let snapshot = Snapshot {
                        fetched_at: OffsetDateTime::now_utc(),
                        res,
                    };
                    match store(&snapshot, pool).await {
                        Ok(()) => info!(
                            fetched_at = %snapshot.fetched_at,
                            "wallet snapshot refresher: cache updated"
                        ),
                        Err(err) => {
                            warn!(%err, "wallet snapshot refresher: failed to persist");
                        }
                    }
                }
                Ok(Ok(Err(err))) => {
                    warn!(%err, "wallet snapshot refresher: fetch failed; cache row untouched");
                }
                Ok(Err(_)) => {
                    warn!("wallet snapshot refresher: worker dropped reply without sending");
                }
                Err(_) => {
                    warn!(
                        "wallet snapshot refresher: timed out after {}s waiting for worker",
                        REFRESH_TIMEOUT.as_secs()
                    );
                }
            }
            false
        }
    }
}

/// Spawns a detached `std::thread` that performs the blocking electrum scan
/// and reports the result back through a `tokio::sync::oneshot` channel.
///
/// The `JoinHandle` is intentionally dropped (thread is detached). The
/// actix tokio runtime is unaware of this thread, so when the runtime
/// drops on SIGTERM it does not wait for the worker. When `main()`
/// returns, Rust calls `std::process::exit()` which terminates all
/// threads — including this one and its stuck TCP connection —
/// regardless of what the electrum call is doing. This is what makes
/// SIGTERM return control within milliseconds even when the electrum
/// server is blackholed.
fn spawn_blocking_refresh(conf: Conf) -> oneshot::Receiver<Result<WalletRes>> {
    let (tx, rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("wallet-refresh".into())
        .spawn(move || {
            let result = aggregate(
                &conf.xpub_spending,
                &conf.xpub_donations,
                &conf.xpub_treasury,
                &conf.electrum_url,
            );
            let _ = tx.send(result);
        })
        .expect("failed to spawn wallet-refresh thread");
    rx
}

pub async fn get_or_fetch(pool: &MainPool) -> Result<Snapshot> {
    if let Some(snapshot) = load(pool).await? {
        return Ok(snapshot);
    }
    let conf = conf_queries::select(pool).await?;
    let rx = spawn_blocking_refresh(conf);
    let res = match tokio::time::timeout(REFRESH_TIMEOUT, rx).await {
        Ok(Ok(Ok(res))) => res,
        Ok(Ok(Err(err))) => return Err(err),
        Ok(Err(_)) => {
            return Err(crate::Error::Other(
                "wallet refresh worker dropped reply without sending".into(),
            ));
        }
        Err(_) => {
            return Err(crate::Error::Other(format!(
                "wallet refresh timed out after {}s",
                REFRESH_TIMEOUT.as_secs()
            )));
        }
    };
    let snapshot = Snapshot {
        fetched_at: OffsetDateTime::now_utc(),
        res,
    };
    if let Err(err) = store(&snapshot, pool).await {
        warn!(%err, "failed to persist wallet snapshot to cache table");
    }
    Ok(snapshot)
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
