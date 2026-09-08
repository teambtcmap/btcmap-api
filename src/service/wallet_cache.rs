use crate::db::main::electrum_server::queries as electrum_server_queries;
use crate::db::main::wallet::schema::CachedTx;
use crate::db::main::wallet::schema::Wallet;
use crate::db::main::MainPool;
use crate::service::wallet::{active_servers_as_tuples, aggregate, Res as WalletRes};
use crate::Result;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Upper bound on how long the cache task will wait for a single wallet refresh
/// to complete. The actual blocking work runs on a detached `std::thread` that
/// the actix tokio runtime does NOT know about, so the process can always exit
/// even if the worker is stuck — this timeout just unblocks the *awaiter* (the
/// cache loop) so it can move on or surface an error.
pub const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

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
    let wallets = match load_active_wallets(pool).await {
        Ok(w) => w,
        Err(err) => {
            warn!(%err, "wallet snapshot refresher: failed to load wallets");
            return false;
        }
    };
    if wallets.is_empty() {
        info!("wallet snapshot refresher: no active wallets, skipping");
        return false;
    }
    let servers = load_active_servers(pool).await;
    let rx = spawn_blocking_refresh(wallets, servers);
    tokio::select! {
        _ = shutdown.cancelled() => true,
        result = tokio::time::timeout(REFRESH_TIMEOUT, rx) => {
            match result {
                Ok(Ok(Ok(res))) => {
                    info!(
                        elapsed = ?started_at.elapsed(),
                        wallet_count = res.wallets.len(),
                        "wallet snapshot refresher: fetch succeeded"
                    );
                    match persist(&res, pool).await {
                        Ok(()) => info!("wallet snapshot refresher: cache updated"),
                        Err(err) => {
                            warn!(%err, "wallet snapshot refresher: failed to persist");
                        }
                    }
                }
                Ok(Ok(Err(err))) => {
                    warn!(%err, "wallet snapshot refresher: fetch failed; cache rows untouched");
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
fn spawn_blocking_refresh(
    wallets: Vec<(i64, String, String)>,
    servers: Vec<(String, String, String)>,
) -> oneshot::Receiver<Result<WalletRes>> {
    let (tx, rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("wallet-refresh".into())
        .spawn(move || {
            let result = aggregate(&wallets, &servers);
            let _ = tx.send(result);
        })
        .expect("failed to spawn wallet-refresh thread");
    rx
}

async fn load_active_servers(pool: &MainPool) -> Vec<(String, String, String)> {
    match electrum_server_queries::select_all(pool).await {
        Ok(servers) => active_servers_as_tuples(servers),
        Err(err) => {
            warn!(%err, "wallet snapshot refresher: failed to load electrum servers");
            Vec::new()
        }
    }
}

async fn load_active_wallets(pool: &MainPool) -> Result<Vec<(i64, String, String)>> {
    let wallets: Vec<Wallet> = crate::db::main::wallet::queries::select_all(pool).await?;
    Ok(wallets
        .into_iter()
        .filter(|w| w.deleted_at.is_none())
        .map(|w| (w.id, w.name, w.xpub))
        .collect())
}

async fn persist(res: &WalletRes, pool: &MainPool) -> Result<()> {
    let cached_at = time::OffsetDateTime::now_utc();
    for snapshot in &res.wallets {
        let tx: Vec<CachedTx> = snapshot
            .tx
            .iter()
            .map(|t| CachedTx {
                id: t.id.clone(),
                received: t.received,
                sent: t.sent,
                delta: t.delta,
            })
            .collect();
        if let Err(err) = crate::db::main::wallet::queries::set_cached_snapshot(
            snapshot.id,
            snapshot.balance_sats,
            tx,
            cached_at,
            pool,
        )
        .await
        {
            warn!(
                wallet = snapshot.name.as_str(),
                %err,
                "wallet snapshot refresher: failed to persist snapshot for wallet"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn load_active_wallets_skips_deleted() -> Result<()> {
        let pool = pool();
        let active = crate::db::main::wallet::queries::insert(
            "spending".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let deleted = crate::db::main::wallet::queries::insert(
            "donations".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        crate::db::main::wallet::queries::set_deleted_at(
            deleted.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let wallets = super::load_active_wallets(&pool).await?;
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].0, active.id);
        assert_eq!(wallets[0].1, "spending");
        Ok(())
    }
}
