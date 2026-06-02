use crate::db::log::request::queries as log_request_queries;
use crate::db::log::LogPool;
use crate::db::main::element_event::queries as element_event_queries;
use crate::db::main::MainPool;
use crate::Result;
use serde::Serialize;
use time::{Duration, OffsetDateTime};

const EVENT_TYPE_CREATE: &str = "create";
const EVENT_TYPE_UPDATE: &str = "update";
const EVENT_TYPE_DELETE: &str = "delete";

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub generation_time_ms: i64,
    pub places: PlaceStats,
    pub logs: LogStats,
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
    };
    let finished_at = OffsetDateTime::now_utc();
    let generation_time_ms = (finished_at - started_at).whole_milliseconds() as i64;
    Ok(Res {
        started_at,
        finished_at,
        generation_time_ms,
        places,
        logs,
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
}
