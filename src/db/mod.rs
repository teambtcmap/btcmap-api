pub mod access_token;
pub mod area;
pub mod area_element;
pub mod ban;
pub mod boost;
pub mod conf;
pub mod element;
pub mod element_comment;
pub mod element_event;
pub mod element_issue;
pub mod event;
pub mod place_submission;
pub mod invoice;
pub mod migration;
pub mod osm_user;
pub mod report;
pub mod user;
use crate::{service::filesystem::data_dir_file_path, Result};
use deadpool_sqlite::{Config, Hook, Pool, Runtime};

pub fn pool() -> Result<Pool> {
    Config::new(data_dir_file_path("btcmap.db")?)
        .builder(Runtime::Tokio1)?
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
    use super::migration;
    use deadpool_sqlite::{Config, Hook, Pool, Runtime};
    use rusqlite::Connection;

    pub fn pool() -> Pool {
        Config::new(":memory:")
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(1)
            .post_create(Hook::Fn(Box::new(|conn, _| {
                let mut conn = conn.lock().unwrap();
                migration::run(&mut conn).unwrap();
                Ok(())
            })))
            .build()
            .unwrap()
    }

    pub(super) fn conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migration::run(&mut conn).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn
    }
}
