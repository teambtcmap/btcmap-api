use crate::db::log::request::queries as log_request_queries;
use crate::db::log::sync::queries as log_sync_queries;
use crate::db::log::LogPool;
use crate::db::main::element_event::queries as element_event_queries;
use crate::db::main::place_submission::queries as place_submission_queries;
use crate::db::main::place_submission::schema::OriginSubmissionCounts;
use crate::db::main::MainPool;
use crate::service::lnd;
use crate::service::lnd::NodeStats;
use crate::service::wallet;
use crate::Result;
use serde::Serialize;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use tracing::warn;

const EVENT_TYPE_CREATE: &str = "create";
const EVENT_TYPE_UPDATE: &str = "update";
const EVENT_TYPE_DELETE: &str = "delete";
const SYNC_RUNS_LIMIT: i64 = 10;

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub generation_time_ms: i64,
    pub places: PlaceStats,
    pub imports: Vec<ImportOriginStats>,
    pub logs: LogStats,
    pub unique_ips_24h: PlatformUniqueIps24h,
    pub storage: StorageStats,
    pub lnd: Option<NodeStats>,
    pub sync_runs: Vec<SyncRun>,
    pub wallets: Wallets,
}

#[derive(Serialize)]
pub struct PlaceStats {
    pub added: PeriodCounts,
    pub updated: PeriodCounts,
    pub deleted: PeriodCounts,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct ImportOriginStats {
    pub origin: String,
    pub total: PeriodCounts,
    pub pending: PeriodCounts,
    pub revoked: PeriodCounts,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct PeriodCounts {
    pub d1: i64,
    pub d7: i64,
    pub d30: i64,
}

#[derive(Serialize)]
pub struct LogStats {
    pub file_size_bytes: u64,
    pub requests: PeriodCounts,
    pub top_rpcs: Vec<TopRpc>,
    pub top_rest_api_calls: Vec<TopRestApiCall>,
}

#[derive(Serialize)]
pub struct TopRpc {
    pub method: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct TopRestApiCall {
    pub method: String,
    pub path: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct SyncRun {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
    pub duration_s: Option<f64>,
    pub overpass_response_time_s: Option<f64>,
    pub elements_affected: i64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub failed_at: Option<OffsetDateTime>,
    pub fail_reason: Option<String>,
}

#[derive(Serialize)]
pub struct Wallets {
    pub spending: i64,
    pub donations: i64,
    pub treasury: i64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct PlatformUniqueIps24h {
    pub web: i64,
    pub android: i64,
    pub ios: i64,
    pub other_humans: i64,
    pub bots: i64,
}

#[derive(Serialize)]
pub struct StorageStats {
    pub disks: Vec<Disk>,
}

#[derive(Serialize)]
pub struct Disk {
    pub device: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_percent: f64,
}

pub async fn run(pool: &MainPool, log_pool: &LogPool) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let now = OffsetDateTime::now_utc();
    let d1 = now.saturating_sub(Duration::days(1));
    let d7 = now.saturating_sub(Duration::days(7));
    let d30 = now.saturating_sub(Duration::days(30));
    let places = PlaceStats {
        added: PeriodCounts {
            d1: element_event_queries::select_count_by_type_since(EVENT_TYPE_CREATE, d1, pool)
                .await?,
            d7: element_event_queries::select_count_by_type_since(EVENT_TYPE_CREATE, d7, pool)
                .await?,
            d30: element_event_queries::select_count_by_type_since(EVENT_TYPE_CREATE, d30, pool)
                .await?,
        },
        updated: PeriodCounts {
            d1: element_event_queries::select_count_by_type_since(EVENT_TYPE_UPDATE, d1, pool)
                .await?,
            d7: element_event_queries::select_count_by_type_since(EVENT_TYPE_UPDATE, d7, pool)
                .await?,
            d30: element_event_queries::select_count_by_type_since(EVENT_TYPE_UPDATE, d30, pool)
                .await?,
        },
        deleted: PeriodCounts {
            d1: element_event_queries::select_count_by_type_since(EVENT_TYPE_DELETE, d1, pool)
                .await?,
            d7: element_event_queries::select_count_by_type_since(EVENT_TYPE_DELETE, d7, pool)
                .await?,
            d30: element_event_queries::select_count_by_type_since(EVENT_TYPE_DELETE, d30, pool)
                .await?,
        },
    };
    let imports = collect_imports(pool, d1, d7, d30).await?;
    let logs = LogStats {
        file_size_bytes: log_db_file_size(),
        requests: PeriodCounts {
            d1: log_request_queries::select_count_since(d1, log_pool).await?,
            d7: log_request_queries::select_count_since(d7, log_pool).await?,
            d30: log_request_queries::select_count_since(d30, log_pool).await?,
        },
        top_rpcs: log_request_queries::select_top_rpc_methods(d1, log_pool)
            .await?
            .into_iter()
            .map(|it| TopRpc {
                method: it.method,
                count: it.count,
            })
            .collect(),
        top_rest_api_calls: log_request_queries::select_top_rest_api_calls(d1, log_pool)
            .await?
            .into_iter()
            .map(|it| TopRestApiCall {
                method: it.method,
                path: it.path,
                count: it.count,
            })
            .collect(),
    };
    let raw_sync_runs = log_sync_queries::select_latest(SYNC_RUNS_LIMIT, log_pool).await?;
    let parse_opt = |value: Option<String>| -> Result<Option<OffsetDateTime>> {
        match value {
            Some(s) => Ok(Some(OffsetDateTime::parse(&s, &Rfc3339)?)),
            None => Ok(None),
        }
    };
    let sync_runs: Vec<SyncRun> = raw_sync_runs
        .into_iter()
        .map(|run| {
            Ok(SyncRun {
                id: run.id,
                started_at: OffsetDateTime::parse(&run.started_at, &Rfc3339)?,
                finished_at: parse_opt(run.finished_at)?,
                duration_s: run.duration_s,
                overpass_response_time_s: run.overpass_response_time_s,
                elements_affected: run.elements_affected,
                elements_created: run.elements_created,
                elements_updated: run.elements_updated,
                elements_deleted: run.elements_deleted,
                failed_at: parse_opt(run.failed_at)?,
                fail_reason: run.fail_reason,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let finished_at = OffsetDateTime::now_utc();
    let generation_time_ms = (finished_at - started_at).whole_milliseconds() as i64;
    let lnd = match lnd::get_node_stats(pool).await {
        Ok(stats) => Some(stats),
        Err(err) => {
            warn!(%err, "failed to fetch lnd node stats");
            None
        }
    };
    let wallets = match wallet::run(pool).await {
        Ok(stats) => Wallets {
            spending: stats.spending,
            donations: stats.donations,
            treasury: stats.treasury,
        },
        Err(err) => {
            warn!(%err, "failed to fetch wallet stats");
            Wallets {
                spending: 0,
                donations: 0,
                treasury: 0,
            }
        }
    };
    let raw_unique_ips = log_request_queries::select_platform_unique_ips_24h(log_pool).await?;
    let unique_ips_24h = PlatformUniqueIps24h {
        web: raw_unique_ips.web,
        android: raw_unique_ips.android,
        ios: raw_unique_ips.ios,
        other_humans: raw_unique_ips.other_humans,
        bots: raw_unique_ips.bots,
    };
    Ok(Res {
        started_at,
        finished_at,
        generation_time_ms,
        places,
        imports,
        logs,
        unique_ips_24h,
        storage: StorageStats {
            disks: storage_stats(),
        },
        lnd,
        sync_runs,
        wallets,
    })
}

async fn collect_imports(
    pool: &MainPool,
    d1: OffsetDateTime,
    d7: OffsetDateTime,
    d30: OffsetDateTime,
) -> Result<Vec<ImportOriginStats>> {
    let d1_counts = place_submission_queries::select_origin_counts_since(d1, pool).await?;
    let d7_counts = place_submission_queries::select_origin_counts_since(d7, pool).await?;
    let d30_counts = place_submission_queries::select_origin_counts_since(d30, pool).await?;
    let mut by_origin: HashMap<String, ImportOriginStats> = HashMap::new();
    let mut ingest =
        |counts: Vec<OriginSubmissionCounts>,
         apply: &dyn Fn(&mut ImportOriginStats, &OriginSubmissionCounts)| {
            for c in counts {
                let entry =
                    by_origin
                        .entry(c.origin.clone())
                        .or_insert_with(|| ImportOriginStats {
                            origin: c.origin.clone(),
                            total: PeriodCounts {
                                d1: 0,
                                d7: 0,
                                d30: 0,
                            },
                            pending: PeriodCounts {
                                d1: 0,
                                d7: 0,
                                d30: 0,
                            },
                            revoked: PeriodCounts {
                                d1: 0,
                                d7: 0,
                                d30: 0,
                            },
                        });
                apply(entry, &c);
            }
        };
    ingest(d1_counts, &|entry, c| {
        entry.total.d1 = c.total;
        entry.pending.d1 = c.pending;
        entry.revoked.d1 = c.revoked;
    });
    ingest(d7_counts, &|entry, c| {
        entry.total.d7 = c.total;
        entry.pending.d7 = c.pending;
        entry.revoked.d7 = c.revoked;
    });
    ingest(d30_counts, &|entry, c| {
        entry.total.d30 = c.total;
        entry.pending.d30 = c.pending;
        entry.revoked.d30 = c.revoked;
    });
    let mut imports: Vec<ImportOriginStats> = by_origin.into_values().collect();
    imports.sort_by(|a, b| a.origin.cmp(&b.origin));
    Ok(imports)
}

fn log_db_file_size() -> u64 {
    crate::db::db_file_path("log.db")
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|m| m.len())
        .unwrap_or(0)
}

fn storage_stats() -> Vec<Disk> {
    let Some(output) = std::process::Command::new("df")
        .args(["-PB1"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
    else {
        return vec![];
    };
    let mut disks = vec![];
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        if !parts[0].starts_with("/dev/") {
            continue;
        }
        let (Ok(total_bytes), Ok(used_bytes), Ok(available_bytes), Ok(used_percent)) = (
            parts[1].parse::<u64>(),
            parts[2].parse::<u64>(),
            parts[3].parse::<u64>(),
            parts[4].trim_end_matches('%').parse::<f64>(),
        ) else {
            continue;
        };
        disks.push(Disk {
            device: parts[0].to_string(),
            mount_point: parts[5..].join(" "),
            total_bytes,
            used_bytes,
            available_bytes,
            used_percent,
        });
    }
    disks
}

#[cfg(test)]
mod test {
    use crate::db::log::test::pool as log_pool;
    use crate::db::main::test::pool;
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn empty_database() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(0, res.places.added.d1);
        assert_eq!(0, res.places.added.d7);
        assert_eq!(0, res.places.added.d30);
        assert_eq!(0, res.places.updated.d1);
        assert_eq!(0, res.places.updated.d7);
        assert_eq!(0, res.places.updated.d30);
        assert_eq!(0, res.places.deleted.d1);
        assert_eq!(0, res.places.deleted.d7);
        assert_eq!(0, res.places.deleted.d30);
        assert!(res.imports.is_empty());
        assert_eq!(0, res.logs.requests.d1);
        assert_eq!(0, res.logs.requests.d7);
        assert_eq!(0, res.logs.requests.d30);
        assert!(res.logs.top_rpcs.is_empty());
        assert!(res.logs.top_rest_api_calls.is_empty());
        assert!(res.sync_runs.is_empty());
        assert!(res.lnd.is_none());
        assert_eq!(0, res.wallets.spending);
        assert_eq!(0, res.wallets.donations);
        assert_eq!(0, res.wallets.treasury);
        assert_eq!(0, res.unique_ips_24h.web);
        assert_eq!(0, res.unique_ips_24h.android);
        assert_eq!(0, res.unique_ips_24h.ios);
        assert_eq!(0, res.unique_ips_24h.other_humans);
        assert_eq!(0, res.unique_ips_24h.bots);
        assert!(res.finished_at >= res.started_at);
        Ok(())
    }

    #[test]
    async fn counts_unique_ips_per_platform_in_last_24h() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();

        async fn insert_now(
            log_pool: &crate::db::log::LogPool,
            ip: String,
            ua: Option<String>,
            path: String,
        ) -> Result<()> {
            log_pool
                .get()
                .await?
                .interact(move |conn| {
                    conn.execute(
                        "INSERT INTO request (ip, user_agent, path, response_code, processing_time_ns, date) VALUES (?1, ?2, ?3, 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                        rusqlite::params![ip, ua, path],
                    )?;
                    Ok::<(), crate::Error>(())
                })
                .await??;
            Ok(())
        }

        async fn insert_offset(
            log_pool: &crate::db::log::LogPool,
            ip: String,
            ua: Option<String>,
            path: String,
            offset: String,
        ) -> Result<()> {
            log_pool
                .get()
                .await?
                .interact(move |conn| {
                    conn.execute(
                        "INSERT INTO request (ip, user_agent, path, response_code, processing_time_ns, date) VALUES (?1, ?2, ?3, 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?4))",
                        rusqlite::params![ip, ua, path, offset],
                    )?;
                    Ok::<(), crate::Error>(())
                })
                .await??;
            Ok(())
        }

        insert_now(
            &log_pool,
            "10.0.0.1".to_string(),
            Some("btcmap.org".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.2".to_string(),
            Some("btcmap.org".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.2".to_string(),
            Some("btcmap.org".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.3".to_string(),
            Some("BTC Map Android 56".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        // Old android fork is still classified as android
        insert_now(
            &log_pool,
            "10.0.0.30".to_string(),
            Some("okhttp/5.0.0-alpha.14".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.4".to_string(),
            Some("BTCMap/19 CFNetwork/1494.0.7 Darwin/23.4.0".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        // other_humans: cli + NULL UA
        insert_now(
            &log_pool,
            "10.0.0.5".to_string(),
            Some("curl/8.5.0".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.6".to_string(),
            None,
            "/v4/places".to_string(),
        )
        .await?;
        // bots
        insert_now(
            &log_pool,
            "10.0.0.7".to_string(),
            Some("Amazonbot/0.1".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        insert_now(
            &log_pool,
            "10.0.0.8".to_string(),
            Some("Mozilla/5.0 (compatible; Googlebot/2.1)".to_string()),
            "/v4/places".to_string(),
        )
        .await?;
        // out-of-window: should be ignored
        insert_offset(
            &log_pool,
            "10.0.0.99".to_string(),
            Some("btcmap.org".to_string()),
            "/v4/places".to_string(),
            "-2 days".to_string(),
        )
        .await?;

        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(2, res.unique_ips_24h.web);
        assert_eq!(2, res.unique_ips_24h.android, "current + old fork");
        assert_eq!(1, res.unique_ips_24h.ios);
        assert_eq!(2, res.unique_ips_24h.other_humans);
        assert_eq!(2, res.unique_ips_24h.bots);
        Ok(())
    }

    #[test]
    async fn counts_events_by_type_and_window() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        let user = crate::db::main::osm_user::queries::insert(
            1,
            crate::service::osm::EditingApiUser::mock(),
            &pool,
        )
        .await?;
        let element_1 = crate::db::main::element::queries::insert(
            crate::service::overpass::OverpassElement::mock(1),
            &pool,
        )
        .await?;
        let element_2 = crate::db::main::element::queries::insert(
            crate::service::overpass::OverpassElement::mock(2),
            &pool,
        )
        .await?;
        let element_3 = crate::db::main::element::queries::insert(
            crate::service::overpass::OverpassElement::mock(3),
            &pool,
        )
        .await?;
        let element_4 = crate::db::main::element::queries::insert(
            crate::service::overpass::OverpassElement::mock(4),
            &pool,
        )
        .await?;
        let old_create =
            crate::db::main::element_event::queries::insert(user.id, element_1.id, "create", &pool)
                .await?;
        let _recent_create =
            crate::db::main::element_event::queries::insert(user.id, element_2.id, "create", &pool)
                .await?;
        let _recent_update =
            crate::db::main::element_event::queries::insert(user.id, element_3.id, "update", &pool)
                .await?;
        let old_delete =
            crate::db::main::element_event::queries::insert(user.id, element_4.id, "delete", &pool)
                .await?;
        pool.get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "UPDATE element_event SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![old_create.id],
                )?;
                conn.execute(
                    "UPDATE element_event SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![old_delete.id],
                )
            })
            .await??;
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(1, res.places.added.d1);
        assert_eq!(1, res.places.added.d7);
        assert_eq!(1, res.places.added.d30);
        assert_eq!(1, res.places.updated.d1);
        assert_eq!(1, res.places.updated.d7);
        assert_eq!(1, res.places.updated.d30);
        assert_eq!(0, res.places.deleted.d1);
        assert_eq!(0, res.places.deleted.d7);
        assert_eq!(0, res.places.deleted.d30);
        Ok(())
    }

    #[test]
    async fn counts_imports_by_origin_and_window() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();

        let insert = |origin: &str, external_id: &str| {
            crate::db::main::place_submission::queries::insert(
                crate::db::main::place_submission::blocking_queries::InsertArgs {
                    origin: origin.to_string(),
                    external_id: external_id.to_string(),
                    lat: 1.0,
                    lon: 2.0,
                    category: "cafe".to_string(),
                    name: "Place".to_string(),
                    extra_fields: serde_json::Map::new(),
                },
                &pool,
            )
        };

        let square_recent_1 = insert("square", "1").await?;
        let square_recent_2 = insert("square", "2").await?;
        let square_old = insert("square", "3").await?;
        let coinos_recent = insert("coinos", "1").await?;
        let coinos_closed = insert("coinos", "2").await?;
        let coinos_revoked = insert("coinos", "3").await?;
        let coinos_old = insert("coinos", "4").await?;

        pool.get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "UPDATE place_submission SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![square_old.id],
                )?;
                conn.execute(
                    "UPDATE place_submission SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![coinos_old.id],
                )?;
                conn.execute(
                    "UPDATE place_submission SET closed_at = '2024-06-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![coinos_closed.id],
                )?;
                conn.execute(
                    "UPDATE place_submission SET revoked = 1 WHERE id = ?1",
                    rusqlite::params![coinos_revoked.id],
                )
            })
            .await??;

        let _ = square_recent_1;
        let _ = square_recent_2;
        let _ = coinos_recent;

        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(
            vec![
                super::ImportOriginStats {
                    origin: "coinos".to_string(),
                    total: super::PeriodCounts {
                        d1: 3,
                        d7: 3,
                        d30: 3
                    },
                    pending: super::PeriodCounts {
                        d1: 1,
                        d7: 1,
                        d30: 1
                    },
                    revoked: super::PeriodCounts {
                        d1: 1,
                        d7: 1,
                        d30: 1
                    },
                },
                super::ImportOriginStats {
                    origin: "square".to_string(),
                    total: super::PeriodCounts {
                        d1: 2,
                        d7: 2,
                        d30: 2
                    },
                    pending: super::PeriodCounts {
                        d1: 2,
                        d7: 2,
                        d30: 2
                    },
                    revoked: super::PeriodCounts {
                        d1: 0,
                        d7: 0,
                        d30: 0
                    },
                },
            ],
            res.imports
        );
        Ok(())
    }

    #[test]
    async fn counts_logged_requests_by_window() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        log_pool
            .get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.1', '/api/v1/recent', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.2', '/api/v1/old', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 days'))",
                    [],
                )
            })
            .await??;
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(1, res.logs.requests.d1);
        assert_eq!(2, res.logs.requests.d7);
        assert_eq!(2, res.logs.requests.d30);
        Ok(())
    }

    #[test]
    async fn returns_top_rpc_methods_in_last_24h() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        log_pool
            .get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO request (ip, path, body, response_code, processing_time_ns, date) VALUES ('10.0.0.1', '/rpc', '{\"jsonrpc\":\"2.0\",\"method\":\"revoke_submitted_place\",\"id\":1,\"params\":{}}', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, body, response_code, processing_time_ns, date) VALUES ('10.0.0.2', '/rpc', '{\"jsonrpc\":\"2.0\",\"method\":\"revoke_submitted_place\",\"id\":1,\"params\":{}}', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, body, response_code, processing_time_ns, date) VALUES ('10.0.0.3', '/rpc', '{\"jsonrpc\":\"2.0\",\"method\":\"get_area_dashboard\",\"id\":1,\"params\":{}}', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, body, response_code, processing_time_ns, date) VALUES ('10.0.0.4', '/rpc', '{\"jsonrpc\":\"2.0\",\"method\":\"revoke_submitted_place\",\"id\":1,\"params\":{}}', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 days'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.5', '/v4/places', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )
            })
            .await??;
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(
            vec![
                ("revoke_submitted_place".to_string(), 2),
                ("get_area_dashboard".to_string(), 1),
            ],
            res.logs
                .top_rpcs
                .into_iter()
                .map(|it| (it.method, it.count))
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    async fn returns_top_rest_api_calls_in_last_24h() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        log_pool
            .get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.1', '/v2/elements', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.2', '/v2/elements', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.3', '/v2/elements', 'POST', 201, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.4', '/v4/places/search', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.5', '/v2/elements', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 days'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.6', '/rpc', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.7', '/og/element/1', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO request (ip, path, method, response_code, processing_time_ns, date) VALUES ('10.0.0.8', '/favicon.ico', 'GET', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [],
                )
            })
            .await??;
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(
            vec![
                ("GET".to_string(), "/v2/elements".to_string(), 2),
                ("POST".to_string(), "/v2/elements".to_string(), 1),
                ("GET".to_string(), "/v4/places/search".to_string(), 1),
            ],
            res.logs
                .top_rest_api_calls
                .into_iter()
                .map(|it| (it.method, it.path, it.count))
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    async fn returns_latest_sync_runs() -> Result<()> {
        use time::format_description::well_known::Rfc3339;
        use time::OffsetDateTime;
        let pool = pool();
        let log_pool = log_pool();
        log_pool
            .get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO sync (started_at, finished_at, duration_s, overpass_response_time_s, elements_affected, elements_created, elements_updated, elements_deleted) VALUES ('2024-01-01T00:00:00.000Z', '2024-01-01T00:00:10.000Z', 10.0, 1.5, 6, 1, 2, 3)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO sync (started_at, finished_at, duration_s, overpass_response_time_s, elements_affected, elements_created, elements_updated, elements_deleted) VALUES ('2024-02-01T00:00:00.000Z', '2024-02-01T00:00:20.000Z', 20.0, 2.5, 12, 4, 5, 3)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO sync (started_at, failed_at, fail_reason) VALUES ('2024-03-01T00:00:00.000Z', '2024-03-01T00:00:05.000Z', 'upstream timeout')",
                    [],
                )
            })
            .await??;
        let res = super::run(&pool, &log_pool).await?;
        assert_eq!(3, res.sync_runs.len());
        let parse = |s: &str| OffsetDateTime::parse(s, &Rfc3339).unwrap();
        let latest = &res.sync_runs[0];
        assert_eq!(parse("2024-03-01T00:00:00Z"), latest.started_at);
        assert_eq!(None, latest.finished_at);
        assert_eq!(None, latest.duration_s);
        assert_eq!(None, latest.overpass_response_time_s);
        assert_eq!(0, latest.elements_affected);
        assert_eq!(0, latest.elements_created);
        assert_eq!(0, latest.elements_updated);
        assert_eq!(0, latest.elements_deleted);
        assert!(latest.failed_at.is_some());
        assert_eq!(Some("upstream timeout"), latest.fail_reason.as_deref());
        let middle = &res.sync_runs[1];
        assert_eq!(parse("2024-02-01T00:00:00Z"), middle.started_at);
        assert!(middle.finished_at.is_some());
        assert_eq!(Some(20.0), middle.duration_s);
        assert_eq!(Some(2.5), middle.overpass_response_time_s);
        assert_eq!(12, middle.elements_affected);
        assert_eq!(4, middle.elements_created);
        assert_eq!(5, middle.elements_updated);
        assert_eq!(3, middle.elements_deleted);
        assert!(middle.failed_at.is_none());
        assert!(middle.fail_reason.is_none());
        let oldest = &res.sync_runs[2];
        assert_eq!(parse("2024-01-01T00:00:00Z"), oldest.started_at);
        assert!(oldest.finished_at.is_some());
        assert_eq!(Some(10.0), oldest.duration_s);
        assert_eq!(6, oldest.elements_affected);
        Ok(())
    }

    #[test]
    async fn storage_stats_only_includes_real_devices() {
        let disks = super::storage_stats();
        for disk in &disks {
            assert!(
                disk.device.starts_with("/dev/"),
                "unexpected device: {}",
                disk.device
            );
            assert!(disk.total_bytes > 0);
            assert!(disk.used_bytes <= disk.total_bytes);
            assert!(disk.available_bytes <= disk.total_bytes);
            assert!(disk.used_percent >= 0.0 && disk.used_percent <= 100.0);
            assert!(!disk.mount_point.is_empty());
        }
    }
}
