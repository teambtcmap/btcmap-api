use crate::{
    db::{self, boost::schema::Boost},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
pub struct Params {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Res {
    total_places_start: i64,
    total_places_end: i64,
    total_places_change: i64,
    verified_places_1y_start: i64,
    verified_places_1y_end: i64,
    verified_places_1y_change: i64,
    days_since_verified_start: i64,
    days_since_verified_end: i64,
    days_since_verified_change: i64,
    boosts: i64,
    boosts_total_days: i64,
    comments: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let reports_start =
        db::report::queries_async::select_by_date(params.start.date(), None, pool).await?;
    let reports_end =
        db::report::queries_async::select_by_date(params.end.date(), None, pool).await?;
    let global_report_start = reports_start.iter().find(|it| it.area_id == 662).unwrap();
    let global_report_end = reports_end.iter().find(|it| it.area_id == 662).unwrap();
    let total_places_start = global_report_start.tags["total_elements"].as_i64().unwrap();
    let total_places_end = global_report_end.tags["total_elements"].as_i64().unwrap();
    let verified_places_1y_start = global_report_start.tags["up_to_date_elements"]
        .as_i64()
        .unwrap();
    let verified_places_1y_end = global_report_end.tags["up_to_date_elements"]
        .as_i64()
        .unwrap();
    let avg_verification_date_start = global_report_start.tags["avg_verification_date"]
        .as_str()
        .unwrap();
    let avg_verification_date_start = OffsetDateTime::parse(avg_verification_date_start, &Rfc3339)?;
    let days_since_verified_start =
        (global_report_start.created_at - avg_verification_date_start).whole_days();
    let avg_verification_date_end = global_report_end.tags["avg_verification_date"]
        .as_str()
        .unwrap();
    let avg_verification_date_end = OffsetDateTime::parse(avg_verification_date_end, &Rfc3339)?;
    let days_since_verified_end =
        (global_report_end.created_at - avg_verification_date_end).whole_days();
    let boosts = db::boost::queries::select_all(pool).await?;
    let boosts: Vec<Boost> = boosts
        .into_iter()
        .filter(|it| it.created_at > params.start && it.created_at < params.end)
        .collect();
    let boosts_total_days = boosts.iter().map(|it| it.duration_days).sum();
    let comments =
        db::element_comment::queries::select_created_between(params.start, params.end, pool)
            .await?;
    let comments: Vec<_> = comments
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    Ok(Res {
        total_places_start,
        total_places_end,
        total_places_change: total_places_end - total_places_start,
        verified_places_1y_start,
        verified_places_1y_end,
        verified_places_1y_change: verified_places_1y_end - verified_places_1y_start,
        days_since_verified_start,
        days_since_verified_end,
        days_since_verified_change: days_since_verified_end - days_since_verified_start,
        boosts: boosts.len() as i64,
        boosts_total_days,
        comments: comments.len() as i64,
    })
}
