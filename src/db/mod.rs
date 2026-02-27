pub mod access_token;
pub mod area;
pub mod area_element;
pub mod ban;
pub mod conf;
pub mod element;
pub mod element_comment;
pub mod element_event;
pub mod element_issue;
pub mod event;
pub mod image;
pub mod invoice;
pub mod log;
pub mod migration;
pub mod osm_user;
pub mod place_submission;
pub mod report;
pub mod user;
use crate::{service::filesystem::data_dir_file_path, Result};
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use rusqlite::Connection;
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

pub fn configure_connection(conn: &Connection) {
    // WAL + NORMAL combination provides good concurrency, good crash safety, decent performance and simple maintenance
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
    // Foreign keys force data integrity
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    // 5 seconds is a common default value in production systems
    // SQLite will make make multiple retries during that time window
    conn.pragma_update(None, "busy_timeout", 5000).unwrap();
}

pub fn pool() -> Result<MainPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    Config::new(data_dir_file_path("btcmap.db")?)
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
                conn.execute_batch(include_str!("../../schema.sql"))
                    .unwrap();
                Ok(())
            })))
            .build()
            .unwrap();
        MainPool::new(inner)
    }

    pub(super) fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../schema.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }
}
