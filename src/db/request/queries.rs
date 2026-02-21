use super::blocking_queries;
use crate::{db::request::LogPool, Result};

pub async fn insert(
    ip: &str,
    user_agent: Option<&str>,
    user_id: Option<i64>,
    path: &str,
    query: Option<&str>,
    body: Option<&str>,
    response_code: i64,
    processing_time_ns: i64,
    pool: &LogPool,
) -> Result<()> {
    let ip = ip.to_owned();
    let user_agent = user_agent.map(|s| s.to_owned());
    let path = path.to_owned();
    let query = query.map(|s| s.to_owned());
    let body = body.map(|s| s.to_owned());
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                &ip,
                user_agent.as_deref(),
                user_id,
                &path,
                query.as_deref(),
                body.as_deref(),
                response_code,
                processing_time_ns,
                conn,
            )
        })
        .await?
}
