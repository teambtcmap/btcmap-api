use std::sync::OnceLock;

pub const TABLE_NAME: &str = "request";

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

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Date => "date",
            Columns::Ip => "ip",
            Columns::UserAgent => "user_agent",
            Columns::UserId => "user_id",
            Columns::Path => "path",
            Columns::Query => "query",
            Columns::Body => "body",
            Columns::ResponseCode => "response_code",
            Columns::ProcessingTimeNs => "processing_time_ns",
        }
    }
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
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&rusqlite::Row) -> rusqlite::Result<Self> {
        |row: &rusqlite::Row| -> rusqlite::Result<Self> {
            Ok(Request {
                id: row.get(Columns::Id.as_str())?,
                date: row.get(Columns::Date.as_str())?,
                ip: row.get(Columns::Ip.as_str())?,
                user_agent: row.get(Columns::UserAgent.as_str())?,
                user_id: row.get(Columns::UserId.as_str())?,
                path: row.get(Columns::Path.as_str())?,
                query: row.get(Columns::Query.as_str())?,
                body: row.get(Columns::Body.as_str())?,
                response_code: row.get(Columns::ResponseCode.as_str())?,
                processing_time_ns: row.get(Columns::ProcessingTimeNs.as_str())?,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Columns;

    #[test]
    fn columns_as_str() {
        assert_eq!(Columns::Id.as_str(), "id");
        assert_eq!(Columns::Date.as_str(), "date");
        assert_eq!(Columns::Ip.as_str(), "ip");
        assert_eq!(Columns::UserAgent.as_str(), "user_agent");
        assert_eq!(Columns::UserId.as_str(), "user_id");
        assert_eq!(Columns::Path.as_str(), "path");
        assert_eq!(Columns::Query.as_str(), "query");
        assert_eq!(Columns::Body.as_str(), "body");
        assert_eq!(Columns::ResponseCode.as_str(), "response_code");
        assert_eq!(Columns::ProcessingTimeNs.as_str(), "processing_time_ns");
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
