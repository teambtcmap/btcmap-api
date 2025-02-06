use crate::{
    area::{self, service::TrendingArea},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
pub struct Params {
    pub period_start: String,
    pub period_end: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<TrendingArea>> {
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_start), &Rfc3339)?;
    let period_end = OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_end), &Rfc3339)?;
    area::service::get_trending_areas_async("community", &period_start, &period_end, &pool).await
}
