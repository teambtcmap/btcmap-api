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
mod test {
    pub(super) fn conn() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db_utils::migrate(&mut conn).unwrap();
        conn
    }
}
