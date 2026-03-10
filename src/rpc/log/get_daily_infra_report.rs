use crate::db::log::request::queries;
use crate::db::log::LogPool;
use crate::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub total_requests: i64,
    pub unique_ips: i64,
    pub web: PlatformStats,
    pub android: PlatformStats,
    pub ios: PlatformStats,
}

#[derive(Serialize)]
pub struct PlatformStats {
    pub requests: i64,
    pub unique_ips: i64,
}

pub async fn run(pool: &LogPool) -> Result<Res> {
    let report = queries::select_daily_infra_report(pool).await?;
    Ok(Res {
        total_requests: report.total_requests,
        unique_ips: report.unique_ips,
        web: PlatformStats {
            requests: report.web_requests,
            unique_ips: report.web_unique_ips,
        },
        android: PlatformStats {
            requests: report.android_requests,
            unique_ips: report.android_unique_ips,
        },
        ios: PlatformStats {
            requests: report.ios_requests,
            unique_ips: report.ios_unique_ips,
        },
    })
}
