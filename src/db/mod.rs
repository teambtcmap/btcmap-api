pub mod element_issue;
pub mod event;
pub mod image;
pub mod invoice;
pub mod log;
pub mod main;
pub mod osm_user;
pub mod place_submission;
pub mod report;
pub mod user;

use crate::Result;
use rusqlite::Connection;
use std::{
    fmt::{Display, Formatter},
    fs::create_dir_all,
    path::PathBuf,
};

pub struct Migration(pub i16, pub String);

impl Display for Migration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {})",
            self.0,
            self.1
                .replace("\n", "")
                .replace("    ", "")
                .replace(";", "; "),
        )
    }
}

pub fn db_file_path(db_name: &str) -> Result<PathBuf> {
    #[allow(deprecated)]
    let data_dir = std::env::home_dir()
        .ok_or("home directory does not exist")?
        .join(".local/share/btcmap");
    if !data_dir.exists() {
        create_dir_all(&data_dir)?;
    }
    Ok(data_dir.join(db_name))
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
