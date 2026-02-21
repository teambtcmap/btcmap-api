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

#[allow(dead_code)]
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
        "id, date, ip, user_agent, user_id, path, query, body, response_code, processing_time_ns"
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
