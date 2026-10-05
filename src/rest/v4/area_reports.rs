use crate::db;
use crate::db::main::report::schema::Report;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

/// Hard cap on `limit`. The report table holds at most one row per area per
/// changed day, so 10,000 rows covers roughly 27 years of daily changes and is
/// far above any real area's history.
const MAX_LIMIT: i64 = 10_000;

#[derive(Deserialize)]
pub struct GetReportsArgs {
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct AreaReportTags {
    #[ts(type = "number")]
    pub total_elements: i64,
    #[ts(type = "number")]
    pub total_elements_onchain: i64,
    #[ts(type = "number")]
    pub total_elements_lightning: i64,
    #[ts(type = "number")]
    pub total_elements_lightning_contactless: i64,
    #[ts(type = "number")]
    pub total_atms: i64,
    #[ts(type = "number")]
    pub total_merchants: i64,
    #[ts(type = "number")]
    pub total_exchanges: i64,
    #[ts(type = "number")]
    pub up_to_date_elements: i64,
    #[ts(type = "number")]
    pub up_to_date_percent: i64,
    #[ts(type = "number")]
    pub outdated_elements: i64,
    #[ts(type = "number")]
    pub legacy_elements: i64,
    /// Average verification date across the area's places, as stored. Omitted
    /// when the row predates the field or no place had a verification date.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string")]
    pub avg_verification_date: Option<String>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct AreaReport {
    #[ts(type = "number")]
    pub id: i64,
    pub date: String,
    pub tags: AreaReportTags,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
}

fn tag_i64(report: &Report, key: &str) -> i64 {
    report
        .tags
        .get(key)
        .and_then(|it| it.as_i64())
        .unwrap_or_default()
}

impl From<&Report> for AreaReport {
    fn from(report: &Report) -> Self {
        Self {
            id: report.id,
            date: report.date.to_string(),
            tags: AreaReportTags {
                // Reuse the schema helpers for the fields with a legacy
                // fallback: `total_merchants` derives from
                // total_elements - total_atms, `total_exchanges` from
                // total_atms, when the stored key is absent.
                total_elements: report.total_elements(),
                total_elements_onchain: tag_i64(report, "total_elements_onchain"),
                total_elements_lightning: tag_i64(report, "total_elements_lightning"),
                total_elements_lightning_contactless: tag_i64(
                    report,
                    "total_elements_lightning_contactless",
                ),
                total_atms: report.total_atms(),
                total_merchants: report.total_merchants(),
                total_exchanges: report.total_exchanges(),
                up_to_date_elements: report.up_to_date_elements(),
                up_to_date_percent: tag_i64(report, "up_to_date_percent"),
                outdated_elements: tag_i64(report, "outdated_elements"),
                legacy_elements: tag_i64(report, "legacy_elements"),
                avg_verification_date: report
                    .tags
                    .get("avg_verification_date")
                    .and_then(|it| it.as_str())
                    .map(str::to_string),
            },
            created_at: report.created_at,
            updated_at: report.updated_at,
        }
    }
}

/// Resolve the `limit` query parameter. Missing means the area's full history;
/// present values are clamped to `1..=MAX_LIMIT` so a caller can't ask for an
/// unbounded set without erroring on a harmless large cap.
fn resolve_limit(limit: Option<i64>) -> Result<Option<i64>, RestApiError> {
    match limit {
        None => Ok(None),
        Some(limit) if limit < 1 => {
            Err(RestApiError::invalid_input("limit must be greater than 0"))
        }
        Some(limit) => Ok(Some(limit.min(MAX_LIMIT))),
    }
}

/// Return one area's daily aggregate report history, oldest first. This is the
/// area report (`/v2/reports` upstream), not a place report.
#[get("{id}/reports")]
pub async fn get_by_area(
    id: Path<String>,
    args: Query<GetReportsArgs>,
    pool: Data<MainPool>,
) -> Res<Vec<AreaReport>> {
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    let area = db::main::area::queries::select_by_id_or_alias(id.into_inner(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let limit = resolve_limit(args.limit)?;
    let reports =
        db::main::report::queries::select_by_area_id_ordered_by_date(area.id, limit, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

    Ok(Json(reports.iter().map(AreaReport::from).collect()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::area::schema::Area;
    use crate::db::main::test::pool;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use geojson::JsonObject;
    use serde_json::json;
    use serde_json::Value;
    use time::macros::date;

    fn tags(pairs: &[(&str, i64)]) -> JsonObject {
        let mut tags = JsonObject::new();
        for (key, value) in pairs {
            tags.insert((*key).to_string(), json!(value));
        }
        tags
    }

    async fn setup() -> crate::Result<(crate::db::main::MainPool, Area)> {
        let pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        Ok((pool, area))
    }

    #[test]
    async fn unknown_area_returns_404() -> crate::Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get().uri("/9999/reports").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    #[test]
    async fn known_area_without_reports_returns_empty_array() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports", area.id))
            .to_request();
        let res: Vec<AreaReport> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn resolves_area_alias() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        db::main::report::queries::insert(
            area.id,
            date!(2024 - 01 - 01),
            tags(&[("total_elements", 10)]),
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get().uri("/alias/reports").to_request();
        let res: Vec<AreaReport> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn scopes_rows_to_the_requested_area() -> crate::Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        let other = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        let mine = db::main::report::queries::insert(
            area.id,
            date!(2024 - 01 - 01),
            tags(&[("total_elements", 1)]),
            &pool,
        )
        .await?;
        db::main::report::queries::insert(
            other.id,
            date!(2024 - 01 - 02),
            tags(&[("total_elements", 2)]),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports", area.id))
            .to_request();
        let res: Vec<AreaReport> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(mine.id, res[0].id);
        Ok(())
    }

    #[test]
    async fn orders_rows_by_date_ascending() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        // Insert out of order, and with a later row that was updated first so
        // an updated_at ordering would not match the date ordering.
        db::main::report::queries::insert(
            area.id,
            date!(2024 - 03 - 01),
            tags(&[("total_elements", 3)]),
            &pool,
        )
        .await?;
        let old = db::main::report::queries::insert(
            area.id,
            date!(2024 - 01 - 01),
            tags(&[("total_elements", 1)]),
            &pool,
        )
        .await?;
        db::main::report::queries::set_updated_at(
            old.id,
            time::macros::datetime!(2025-01-01 0:00 UTC),
            &pool,
        )
        .await?;
        db::main::report::queries::insert(
            area.id,
            date!(2024 - 02 - 01),
            tags(&[("total_elements", 2)]),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports", area.id))
            .to_request();
        let res: Vec<AreaReport> = test::call_and_read_body_json(&app, req).await;
        let dates: Vec<&str> = res.iter().map(|it| it.date.as_str()).collect();
        assert_eq!(vec!["2024-01-01", "2024-02-01", "2024-03-01"], dates);
        Ok(())
    }

    #[test]
    async fn honors_limit() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        for day in 1..=5 {
            db::main::report::queries::insert(
                area.id,
                time::Date::from_calendar_date(2024, time::Month::January, day).unwrap(),
                tags(&[("total_elements", day as i64)]),
                &pool,
            )
            .await?;
        }
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports?limit=2", area.id))
            .to_request();
        let res: Vec<AreaReport> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        // The oldest rows are kept and returned chronologically.
        assert_eq!(
            vec!["2024-01-01", "2024-01-02"],
            res.iter().map(|it| it.date.as_str()).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    async fn rejects_non_positive_limit() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports?limit=0", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn legacy_tags_default_missing_keys() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        // Legacy row: no total_merchants, total_exchanges, total_atms, no
        // avg_verification_date, and missing several newer keys.
        db::main::report::queries::insert(
            area.id,
            date!(2023 - 01 - 01),
            tags(&[("total_elements", 245), ("up_to_date_percent", 85)]),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports", area.id))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        let tags = &res[0]["tags"];

        assert_eq!(245, tags["total_elements"].as_i64().unwrap());
        assert_eq!(85, tags["up_to_date_percent"].as_i64().unwrap());
        // Missing keys default to 0 instead of panicking.
        assert_eq!(0, tags["total_elements_onchain"].as_i64().unwrap());
        assert_eq!(0, tags["total_elements_lightning"].as_i64().unwrap());
        assert_eq!(
            0,
            tags["total_elements_lightning_contactless"]
                .as_i64()
                .unwrap()
        );
        assert_eq!(0, tags["outdated_elements"].as_i64().unwrap());
        assert_eq!(0, tags["legacy_elements"].as_i64().unwrap());
        assert_eq!(0, tags["total_atms"].as_i64().unwrap());
        // Legacy fallbacks: total_merchants = total_elements - total_atms,
        // total_exchanges = total_atms.
        assert_eq!(245, tags["total_merchants"].as_i64().unwrap());
        assert_eq!(0, tags["total_exchanges"].as_i64().unwrap());
        // avg_verification_date is omitted when absent.
        assert!(tags.get("avg_verification_date").is_none());
        // area_id is scoped by the path and not echoed back.
        assert!(res[0].get("area_id").is_none());
        Ok(())
    }

    #[test]
    async fn preserves_present_optional_tag() -> crate::Result<()> {
        let (pool, area) = setup().await?;
        let mut row = tags(&[
            ("total_elements", 10),
            ("total_atms", 2),
            ("total_merchants", 8),
            ("total_exchanges", 2),
            ("up_to_date_percent", 50),
        ]);
        row.insert(
            "avg_verification_date".to_string(),
            json!("2026-03-14T00:00:00.000000000Z"),
        );
        db::main::report::queries::insert(area.id, date!(2026 - 04 - 30), row, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/reports", area.id))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(
            "2026-03-14T00:00:00.000000000Z",
            res[0]["tags"]["avg_verification_date"].as_str().unwrap()
        );
        Ok(())
    }
}
