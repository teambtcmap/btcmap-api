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
    pub top_user_agents: Vec<TopUserAgent>,
}

#[derive(Serialize)]
pub struct PlatformStats {
    pub requests: i64,
    pub unique_ips: i64,
}

#[derive(Serialize)]
pub struct TopUserAgent {
    pub user_agent: String,
    pub count: i64,
    pub unique_ips: i64,
}

pub async fn run(pool: &LogPool) -> Result<Res> {
    let report = queries::select_daily_infra_report(pool).await?;
    let top_user_agents = queries::select_top_user_agents(pool).await?;
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
        top_user_agents: top_user_agents
            .into_iter()
            .map(|ua| TopUserAgent {
                user_agent: ua.user_agent,
                count: ua.count,
                unique_ips: ua.unique_ips,
            })
            .collect(),
    })
}
