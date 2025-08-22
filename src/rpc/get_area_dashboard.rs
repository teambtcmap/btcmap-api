use crate::{
    db::{self, report::schema::Report},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub area_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub total_elements: i64,
    pub verified_elements_365d: i64,
    pub total_elements_chart: Vec<ChartEntry>,
    pub verified_elements_365d_chart: Vec<ChartEntry>,
    pub days_since_verified_chart: Vec<ChartEntry>,
}

#[derive(Serialize)]
pub struct ChartEntry {
    pub date: String,
    pub value: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let area = db::area::queries::select_by_id(params.area_id, pool).await?;
    let mut reports = db::report::queries::select_by_area_id(area.id, None, pool).await?;
    reports.sort_by(|a, b| b.date.cmp(&a.date));
    let reports: Vec<Report> = reports.into_iter().take(365).collect();
    let Some(latest_report) = reports.first() else {
        return Err("No data".into());
    };
    let total_elements_chart: Vec<ChartEntry> = reports
        .iter()
        .map(|it| ChartEntry {
            date: it.date.to_string(),
            value: it.total_elements(),
        })
        .collect();
    let verified_elements_365d_chart: Vec<ChartEntry> = reports
        .iter()
        .map(|it| ChartEntry {
            date: it.date.to_string(),
            value: it.up_to_date_elements(),
        })
        .collect();
    let days_since_verified_chart: Vec<ChartEntry> = reports
        .iter()
        .map(|it| ChartEntry {
            date: it.date.to_string(),
            value: it.days_since_verified(),
        })
        .collect();
    Ok(Res {
        total_elements: latest_report.total_elements(),
        verified_elements_365d: latest_report.up_to_date_elements(),
        total_elements_chart,
        verified_elements_365d_chart,
        days_since_verified_chart,
    })
}
