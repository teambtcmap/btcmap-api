use crate::{
    db::{self, main::wallet::schema::Wallet},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub name: String,
    pub xpub: String,
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

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let wallet = db::main::wallet::queries::insert(params.name, params.xpub, pool).await?;
    Ok(wallet.into())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn add_wallet() -> Result<()> {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            xpub: "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        let res = super::run(params, &pool).await?;
        assert_eq!(res.name, "foo");
        assert_eq!(
            res.xpub,
            "xpub0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(res.cached_balance_sats, 0);
        assert!(res.cached_tx.is_empty());
        assert!(res.cached_at.is_none());
        assert!(res.deleted_at.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn add_wallet_rejects_duplicate_name() {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            xpub: "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        super::run(params, &pool).await.unwrap();
        let params = super::Params {
            name: "foo".into(),
            xpub: "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        let res = super::run(params, &pool).await;
        assert!(res.is_err());
    }
}
