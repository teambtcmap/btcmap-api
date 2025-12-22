use geojson::JsonObject;
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "rpc_call";

pub enum Columns {
    Id,
    UserId,
    Ip,
    Method,
    ParamsJson,
    CreatedAt,
    ProcessedAt,
    DurationNs,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::UserId => "user_id",
            Columns::Ip => "ip",
            Columns::Method => "method",
            Columns::ParamsJson => "params_json",
            Columns::CreatedAt => "created_at",
            Columns::ProcessedAt => "processed_at",
            Columns::DurationNs => "duration_ns",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCall {
    pub id: i64,
    pub user_id: Option<i64>,
    pub ip: String,
    pub method: String,
    pub params_json: Option<JsonObject>,
    pub created_at: OffsetDateTime,
    pub processed_at: OffsetDateTime,
    pub duration_ns: i64,
}

impl RpcCall {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::UserId,
                Columns::Ip,
                Columns::Method,
                Columns::ParamsJson,
                Columns::CreatedAt,
                Columns::ProcessedAt,
                Columns::DurationNs,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<RpcCall> {
        |row: &_| {
            let params_json: Option<String> = row.get(Columns::ParamsJson.as_str())?;
            let params_json = match params_json {
                Some(json_str) => match serde_json::from_str(&json_str) {
                    Ok(json_obj) => Some(json_obj),
                    Err(e) => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        ));
                    }
                },
                None => None,
            };
            Ok(RpcCall {
                id: row.get(Columns::Id.as_str())?,
                user_id: row.get(Columns::UserId.as_str())?,
                ip: row.get(Columns::Ip.as_str())?,
                method: row.get(Columns::Method.as_str())?,
                params_json,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                processed_at: row.get(Columns::ProcessedAt.as_str())?,
                duration_ns: row.get(Columns::DurationNs.as_str())?,
            })
        }
    }
}
