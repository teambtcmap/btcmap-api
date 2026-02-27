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
pub mod main;
pub mod migration;
pub mod osm_user;
pub mod place_submission;
pub mod report;
pub mod user;
use rusqlite::Connection;

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
