use super::blocking_queries;
use crate::Result;
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use std::sync::Arc;

#[derive(Clone)]
pub struct LogPool(Arc<Pool>);

impl LogPool {
    pub fn new(pool: Pool) -> Self {
        Self(Arc::new(pool))
    }
}

impl std::ops::Deref for LogPool {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn pool() -> Result<LogPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    let inner = Config::new(crate::service::filesystem::data_dir_file_path("log.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
            blocking_queries::migrate(&conn).unwrap();
            Ok(())
        })))
        .build()?;
    Ok(LogPool::new(inner))
}

pub async fn insert(
    ip: &str,
    user_agent: Option<&str>,
    path: &str,
    query: &str,
    code: i64,
    time_ns: i64,
    pool: &Pool,
) -> Result<()> {
    let ip = ip.to_owned();
    let user_agent = user_agent.map(|s| s.to_owned());
    let path = path.to_owned();
    let query = query.to_owned();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                &ip,
                user_agent.as_deref(),
                &path,
                &query,
                code,
                time_ns,
                conn,
            )
        })
        .await?
}
