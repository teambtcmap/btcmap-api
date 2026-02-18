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
pub mod invoice;
pub mod migration;
pub mod osm_user;
pub mod place_submission;
pub mod report;
pub mod rpc_call;
pub mod user;
use crate::{service::filesystem::data_dir_file_path, Result};
use deadpool_sqlite::{Config, Hook, Pool, Runtime};

pub fn pool() -> Result<Pool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    Config::new(data_dir_file_path("btcmap.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            // WAL + NORMAL combination provides good concurrency, good crash safety, decent performance and simple maintenance
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
            // Foreign keys force data integrity
            conn.pragma_update(None, "foreign_keys", "ON").unwrap();
            // 5 seconds is a common default value in production systems
            // SQLite will make make multiple retries during that time window
            conn.pragma_update(None, "busy_timeout", 5000).unwrap();
            Ok(())
        })))
        .build()
        .map_err(Into::into)
}

#[cfg(test)]
pub mod test {
    use deadpool_sqlite::{Config, Hook, Pool, Runtime};
    use rusqlite::Connection;

    pub fn pool() -> Pool {
        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get() * 2)
            .unwrap_or(8);
        Config::new(":memory:")
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
            .unwrap()
    }

    pub(super) fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../schema.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }
}
