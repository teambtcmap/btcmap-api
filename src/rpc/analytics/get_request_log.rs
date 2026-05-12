use crate::db::log::request::queries;
use crate::db::log::LogPool;
use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    #[serde(default = "default_minutes")]
    pub minutes: i64,
}

fn default_minutes() -> i64 {
    1
}

#[derive(Serialize)]
pub struct Res {
    pub requests: Vec<Request>,
}

#[derive(Serialize)]
pub struct Request {
    pub id: i64,
    pub date: String,
    pub ip: String,
    pub user_agent: Option<String>,
    pub user_id: Option<i64>,
    pub path: String,
    pub query: Option<String>,
    pub body: Option<String>,
    pub response_code: i64,
    pub processing_time_ns: i64,
}

pub async fn run(params: Params, pool: &LogPool) -> Result<Res> {
    let requests = queries::select_latest(params.minutes, pool).await?;
    let requests = requests
        .into_iter()
        .map(|r| Request {
            id: r.id,
            date: r.date,
            ip: r.ip,
            user_agent: r.user_agent,
            user_id: r.user_id,
            path: r.path,
            query: r.query,
            body: r.body,
            response_code: r.response_code,
            processing_time_ns: r.processing_time_ns,
        })
        .collect();
    Ok(Res { requests })
}
