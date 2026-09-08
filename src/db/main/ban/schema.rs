use rusqlite::Row;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "ban";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Ip,
    Reason,
    StartAt,
    EndAt,
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
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Ban> {
        |row| {
            Ok(Ban {
                id: row.get(Columns::Id.as_ref())?,
                ip: row.get(Columns::Ip.as_ref())?,
                reason: row.get(Columns::Reason.as_ref())?,
                start_at: row.get(Columns::StartAt.as_ref())?,
                end_at: row.get(Columns::EndAt.as_ref())?,
            })
        }
    }
}
