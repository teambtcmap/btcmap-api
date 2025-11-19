use actix_web::{
    get,
    web::{Data, Json},
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::{Duration, OffsetDateTime};

use crate::{
    db,
    rest::error::{RestApiError, RestResult},
};

#[derive(Serialize)]
pub struct Dashboard {
    pub total_merchants: i64,
    pub total_merchants_chart: Vec<ChartEntry>,
    pub verified_merchants_1y: i64,
    pub verified_merchants_1y_chart: Vec<ChartEntry>,
    pub total_exchanges: i64,
    pub total_exchanges_chart: Vec<ChartEntry>,
    pub verified_exchanges_1y: i64,
}

#[derive(Serialize)]
pub struct ChartEntry {
    pub date: String,
    pub value: i64,
}

#[get("")]
pub async fn get(pool: Data<Pool>) -> RestResult<Dashboard> {
    let total_merchants = db::element::queries::select_merchants_count(&pool, None)
        .await
        .map_err(|_| RestApiError::database())?;

    let now = OffsetDateTime::now_utc();
    let year_ago = now.date().saturating_sub(Duration::days(365));
    let verified_merchants_1y = db::element::queries::select_merchants_count(&pool, Some(year_ago))
        .await
        .map_err(|_| RestApiError::database())?;
    let verified_exchanges_1y = db::element::queries::select_exchanges_count(&pool, Some(year_ago))
        .await
        .map_err(|_| RestApiError::database())?;

    let total_exchanges = db::element::queries::select_exchanges_count(&pool, None)
        .await
        .map_err(|_| RestApiError::database())?;

    let reports = db::report::queries::select_by_area_id(662, None, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let total_merchants_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.total_elements() - report.total_atms(),
        })
        .collect();

    let verified_merchants_1y_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.up_to_date_elements(),
        })
        .collect();

    let total_exchanges_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.total_atms(),
        })
        .collect();

    Ok(Json(Dashboard {
        total_merchants,
        total_merchants_chart,
        verified_merchants_1y,
        verified_merchants_1y_chart,
        total_exchanges,
        total_exchanges_chart,
        verified_exchanges_1y,
    }))
}
