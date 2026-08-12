use rusqlite::Row;
use serde::Deserialize;
use serde::Serialize;
use std::sync::OnceLock;
use time::OffsetDateTime;

pub const TABLE_NAME: &str = "wallet";

#[allow(non_camel_case_types)]
#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Name,
    Xpub,
    CachedBalanceSats,
    CachedTx,
    CachedAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedTx {
    pub id: String,
    pub received: i64,
    pub sent: i64,
    pub delta: i64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Wallet {
    pub id: i64,
    pub name: String,
    pub xpub: String,
    pub cached_balance_sats: i64,
    pub cached_tx: Vec<CachedTx>,
    pub cached_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Wallet {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Name,
                Columns::Xpub,
                Columns::CachedBalanceSats,
                Columns::CachedTx,
                Columns::CachedAt,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            let cached_tx: String = row.get(Columns::CachedTx.as_ref())?;
            let cached_tx: Vec<CachedTx> = serde_json::from_str(&cached_tx).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Wallet {
                id: row.get(Columns::Id.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                xpub: row.get(Columns::Xpub.as_ref())?,
                cached_balance_sats: row.get(Columns::CachedBalanceSats.as_ref())?,
                cached_tx,
                cached_at: row.get(Columns::CachedAt.as_ref())?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }
}
