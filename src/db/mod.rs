pub mod access_token;
pub mod area;
pub mod area_element;
pub mod ban;
pub mod boost;
pub mod conf;
pub mod element;
pub mod element_comment;
pub mod element_issue;
pub mod event;
pub mod invoice;
pub mod osm_user;
pub mod report;
pub mod user;

#[cfg(test)]
pub mod test {
    use crate::db_utils;
    use deadpool_sqlite::{Config, Hook, Pool, Runtime};
    use rusqlite::Connection;

    pub fn pool() -> Pool {
        Config::new(":memory:")
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(1)
            .post_create(Hook::Fn(Box::new(|conn, _| {
                let mut conn = conn.lock().unwrap();
                db_utils::migrate(&mut conn).unwrap();
                Ok(())
            })))
            .build()
            .unwrap()
    }

    pub(super) fn conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db_utils::migrate(&mut conn).unwrap();
        conn
    }
}
