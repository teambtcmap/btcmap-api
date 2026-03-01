use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "ban";

pub enum Columns {
    Id,
    Ip,
    Reason,
    StartAt,
    EndAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Ip => "ip",
            Columns::Reason => "reason",
            Columns::StartAt => "start_at",
            Columns::EndAt => "end_at",
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub struct Ban {
    pub id: i64,
    pub ip: String,
    pub reason: String,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
}

impl Ban {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Ip,
                Columns::Reason,
                Columns::StartAt,
                Columns::EndAt,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Ban> {
        |row| {
            Ok(Ban {
                id: row.get(Columns::Id.as_str())?,
                ip: row.get(Columns::Ip.as_str())?,
                reason: row.get(Columns::Reason.as_str())?,
                start_at: row.get(Columns::StartAt.as_str())?,
                end_at: row.get(Columns::EndAt.as_str())?,
            })
        }
    }
}
