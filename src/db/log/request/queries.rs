use super::super::LogPool;
use super::blocking_queries;
use super::blocking_queries::{DailyInfraReport, InsertArgs};
use crate::db::log::request::schema::Request;
use crate::Result;

pub async fn insert(args: InsertArgs, pool: &LogPool) -> Result<()> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(args, conn))
        .await?
}

#[allow(dead_code)]
pub async fn select_latest(minutes: i64, pool: &LogPool) -> Result<Vec<Request>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_latest(minutes, conn))
        .await?
}

pub async fn select_daily_infra_report(pool: &LogPool) -> Result<DailyInfraReport> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_daily_infra_report(conn))
        .await?
}
