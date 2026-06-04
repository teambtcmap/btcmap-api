use std::sync::OnceLock;

pub const TABLE_NAME: &str = "request";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Date,
    Ip,
    UserAgent,
    UserId,
    Path,
    Query,
    Body,
    ResponseCode,
    ProcessingTimeNs,
}

pub struct Request {
    pub id: i64,
    pub date: String,
    pub ip: String,
    pub user_agent: Option<String>,
    pub user_id: Option<i64>,
    pub path: String,
    pub query: Option<String>,
    pub body: Option<String>,
    pub response_code: i64,
    pub processing_time_ns: i64,
}

#[allow(dead_code)]
impl Request {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Date,
                Columns::Ip,
                Columns::UserAgent,
                Columns::UserId,
                Columns::Path,
                Columns::Query,
                Columns::Body,
                Columns::ResponseCode,
                Columns::ProcessingTimeNs,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&rusqlite::Row) -> rusqlite::Result<Self> {
        |row: &rusqlite::Row| -> rusqlite::Result<Self> {
            Ok(Request {
                id: row.get(Columns::Id.as_ref())?,
                date: row.get(Columns::Date.as_ref())?,
                ip: row.get(Columns::Ip.as_ref())?,
                user_agent: row.get(Columns::UserAgent.as_ref())?,
                user_id: row.get(Columns::UserId.as_ref())?,
                path: row.get(Columns::Path.as_ref())?,
                query: row.get(Columns::Query.as_ref())?,
                body: row.get(Columns::Body.as_ref())?,
                response_code: row.get(Columns::ResponseCode.as_ref())?,
                processing_time_ns: row.get(Columns::ProcessingTimeNs.as_ref())?,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Columns;

    #[test]
    fn columns_as_str() {
        assert_eq!(Columns::Id.as_ref(), "id");
        assert_eq!(Columns::Date.as_ref(), "date");
        assert_eq!(Columns::Ip.as_ref(), "ip");
        assert_eq!(Columns::UserAgent.as_ref(), "user_agent");
        assert_eq!(Columns::UserId.as_ref(), "user_id");
        assert_eq!(Columns::Path.as_ref(), "path");
        assert_eq!(Columns::Query.as_ref(), "query");
        assert_eq!(Columns::Body.as_ref(), "body");
        assert_eq!(Columns::ResponseCode.as_ref(), "response_code");
        assert_eq!(Columns::ProcessingTimeNs.as_ref(), "processing_time_ns");
    }

    #[test]
    fn request_projection() {
        let projection = super::Request::projection();
        assert!(projection.contains("id"));
        assert!(projection.contains("date"));
        assert!(projection.contains("ip"));
        assert!(projection.contains("user_agent"));
        assert!(projection.contains("user_id"));
        assert!(projection.contains("path"));
        assert!(projection.contains("query"));
        assert!(projection.contains("body"));
        assert!(projection.contains("response_code"));
        assert!(projection.contains("processing_time_ns"));
    }
}
