use crate::db::log::request::queries;
use crate::db::log::LogPool;
use crate::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub total_requests: i64,
    pub unique_ips: i64,
    pub top_ips: Vec<TopIp>,
}

#[derive(Serialize)]
pub struct TopIp {
    pub ip: String,
    pub count: i64,
}

pub async fn run(pool: &LogPool) -> Result<Vec<Res>> {
    let report = queries::select_top_clients(pool).await?;
    let mut platforms = vec![
        (
            "web",
            report.web.total_requests,
            report.web.unique_ips,
            report.web.top_ips,
        ),
        (
            "android",
            report.android.total_requests,
            report.android.unique_ips,
            report.android.top_ips,
        ),
        (
            "ios",
            report.ios.total_requests,
            report.ios.unique_ips,
            report.ios.top_ips,
        ),
    ];
    platforms.sort_by_key(|b| std::cmp::Reverse(b.2));
    Ok(platforms
        .into_iter()
        .map(|(name, total_requests, unique_ips, top_ips)| Res {
            name: name.to_string(),
            total_requests,
            unique_ips,
            top_ips: top_ips
                .into_iter()
                .map(|ip| TopIp {
                    ip: ip.ip,
                    count: ip.count,
                })
                .collect(),
        })
        .collect())
}
