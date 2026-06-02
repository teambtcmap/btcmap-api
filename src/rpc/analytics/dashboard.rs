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

pub async fn run(pool: &MainPool) -> Result<Res> {
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
    let finished_at = OffsetDateTime::now_utc();
    let generation_time_ms = (finished_at - started_at).whole_milliseconds() as i64;
    Ok(Res {
        started_at,
        finished_at,
        generation_time_ms,
        places,
    })
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn empty_database() -> Result<()> {
        let pool = pool();
        let res = super::run(&pool).await?;
        assert_eq!(0, res.places.added.d1);
        assert_eq!(0, res.places.added.d7);
        assert_eq!(0, res.places.added.d30);
        assert_eq!(0, res.places.updated.d1);
        assert_eq!(0, res.places.updated.d7);
        assert_eq!(0, res.places.updated.d30);
        assert_eq!(0, res.places.deleted.d1);
        assert_eq!(0, res.places.deleted.d7);
        assert_eq!(0, res.places.deleted.d30);
        assert!(res.finished_at >= res.started_at);
        Ok(())
    }

    #[test]
    async fn counts_events_by_type_and_window() -> Result<()> {
        let pool = pool();
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
        let res = super::run(&pool).await?;
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
}
