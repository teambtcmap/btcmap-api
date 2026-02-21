pub(super) mod blocking_queries;
pub(super) mod migrations;
pub mod queries;
pub mod schema;

use crate::{service::filesystem::data_dir_file_path, Result};
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

pub fn log_pool() -> Result<LogPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    let inner = Config::new(data_dir_file_path("log.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
            crate::db::request::migrations::v0_to_v1(&conn).unwrap();
            Ok(())
        })))
        .build()?;
    Ok(LogPool::new(inner))
}

#[cfg(test)]
pub mod test {
    use super::LogPool;
    use deadpool_sqlite::{Config, Hook, Runtime};

    pub fn log_pool() -> LogPool {
        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get() * 2)
            .unwrap_or(8);
        let pool = Config::new(":memory:")
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(pool_size)
            .post_create(Hook::Fn(Box::new(|conn, _| {
                let conn = conn.lock().unwrap();
                crate::db::request::migrations::v0_to_v1(&conn).unwrap();
                Ok(())
            })))
            .build()
            .unwrap();
        LogPool::new(pool)
    }
}
