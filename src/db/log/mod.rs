mod migrations;
pub mod request;

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

pub fn pool() -> Result<LogPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    let inner = Config::new(data_dir_file_path("log.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            super::configure_connection(&conn);
            migrations::v0_to_v1(&conn).unwrap();
            Ok(())
        })))
        .build()?;
    Ok(LogPool::new(inner))
}

#[cfg(test)]
pub mod test {
    use super::super::log::migrations;
    use rusqlite::Connection;

    pub fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        super::super::configure_connection(&conn);
        migrations::v0_to_v1(&conn).unwrap();
        conn
    }
}
