use crate::db::configure_connection;
use crate::Result;
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use std::sync::Arc;

#[derive(Clone)]
pub struct MainPool(Arc<Pool>);

impl MainPool {
    pub fn new(pool: Pool) -> Self {
        Self(Arc::new(pool))
    }
}

impl std::ops::Deref for MainPool {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn pool() -> Result<MainPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    Config::new(super::db_file_path("btcmap.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            configure_connection(&conn);
            Ok(())
        })))
        .build()
        .map_err(Into::into)
        .map(MainPool::new)
}

#[cfg(test)]
pub mod test {
    use super::MainPool;
    use deadpool_sqlite::{Config, Hook, Runtime};
    use rusqlite::Connection;

    pub fn pool() -> MainPool {
        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get() * 2)
            .unwrap_or(8);
        let inner = Config::new(":memory:")
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(pool_size)
            .post_create(Hook::Fn(Box::new(|conn, _| {
                let conn = conn.lock().unwrap();
                conn.execute_batch(include_str!("../../../schema.sql"))
                    .unwrap();
                Ok(())
            })))
            .build()
            .unwrap();
        MainPool::new(inner)
    }

    pub fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../../schema.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }
}
