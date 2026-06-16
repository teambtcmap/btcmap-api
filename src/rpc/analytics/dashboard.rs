use crate::db::log::request::queries as log_request_queries;
use crate::db::log::sync::queries as log_sync_queries;
use crate::db::log::LogPool;
use crate::db::main::element_event::queries as element_event_queries;
use crate::db::main::MainPool;
use crate::Result;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

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
    pub logs: LogStats,
    pub sync_runs: Vec<SyncRun>,
}

#[derive(Serialize)]
pub struct PlaceStats {
    pub added: PeriodCounts,
    pub updated: PeriodCounts,
    pub deleted: PeriodCounts,
}

#[derive(Serialize)]
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
}

#[derive(Serialize)]
pub struct TopRpc {
    pub method: String,
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
    Ok(Res {
        started_at,
        finished_at,
        generation_time_ms,
        places,
        logs,
        sync_runs,
    })
}

fn log_db_file_size() -> u64 {
    crate::db::db_file_path("log.db")
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|m| m.len())
        .unwrap_or(0)
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
        assert_eq!(0, res.logs.requests.d1);
        assert_eq!(0, res.logs.requests.d7);
        assert_eq!(0, res.logs.requests.d30);
        assert!(res.logs.top_rpcs.is_empty());
        assert!(res.sync_runs.is_empty());
        assert!(res.finished_at >= res.started_at);
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
}
