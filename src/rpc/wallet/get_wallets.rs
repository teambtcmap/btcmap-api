use crate::{
    db::{self, main::wallet::schema::Wallet},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    include_deleted: Option<bool>,
}

#[derive(Serialize)]
pub struct CachedTx {
    pub id: String,
    pub received: i64,
    pub sent: i64,
    pub delta: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub name: String,
    pub xpub: String,
    pub cached_balance_sats: i64,
    pub cached_tx: Vec<CachedTx>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub cached_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Wallet> for Res {
    fn from(wallet: Wallet) -> Self {
        let cached_tx = wallet
            .cached_tx
            .into_iter()
            .map(|t| CachedTx {
                id: t.id,
                received: t.received,
                sent: t.sent,
                delta: t.delta,
            })
            .collect();
        Res {
            id: wallet.id,
            name: wallet.name,
            xpub: wallet.xpub,
            cached_balance_sats: wallet.cached_balance_sats,
            cached_tx,
            cached_at: wallet.cached_at,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
            deleted_at: wallet.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    let include_deleted = params.include_deleted.unwrap_or(false);
    let wallets = db::main::wallet::queries::select_all(pool).await?;
    let wallets: Vec<Wallet> = wallets
        .into_iter()
        .filter(|it| include_deleted || it.deleted_at.is_none())
        .collect();
    Ok(wallets.into_iter().map(Into::into).collect())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn get_wallets_returns_empty_by_default() -> Result<()> {
        let pool = pool();
        let res = super::run(
            super::Params {
                include_deleted: None,
            },
            &pool,
        )
        .await?;
        assert!(res.is_empty());
        Ok(())
    }

    #[actix_web::test]
    async fn get_wallets_returns_active_only_by_default() -> Result<()> {
        let pool = pool();
        let _ = crate::db::main::wallet::queries::insert(
            "spending".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let b = crate::db::main::wallet::queries::insert(
            "donations".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        crate::db::main::wallet::queries::set_deleted_at(
            b.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                include_deleted: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "spending");
        let res = super::run(
            super::Params {
                include_deleted: Some(true),
            },
            &pool,
        )
        .await?;
        assert_eq!(res.len(), 2);
        Ok(())
    }

    #[actix_web::test]
    async fn get_wallets_returns_cached_snapshot() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "spending".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let cached_at = time::OffsetDateTime::now_utc();
        crate::db::main::wallet::queries::set_cached_snapshot(
            wallet.id,
            12345,
            vec![crate::db::main::wallet::schema::CachedTx {
                id: "tx1".into(),
                received: 100,
                sent: 30,
                delta: 70,
            }],
            cached_at,
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                include_deleted: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].cached_balance_sats, 12345);
        assert_eq!(res[0].cached_tx.len(), 1);
        assert_eq!(res[0].cached_tx[0].id, "tx1");
        assert!(res[0].cached_at.is_some());
        Ok(())
    }
}
