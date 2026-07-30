use crate::{
    db::{self, main::wallet::schema::Wallet},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Deserializer, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub xpub: Option<String>,
    /// Tri-state: `None` (field absent) leaves `deleted_at` unchanged;
    /// `Some(None)` (explicit `null`) clears the soft-delete; `Some(Some(t))`
    /// sets `deleted_at` to the given timestamp.
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub deleted_at: Option<Option<OffsetDateTime>>,
}

fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
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
    let wallet =
        db::main::wallet::queries::update(params.id, params.name, params.xpub, pool).await?;
    let wallet = if let Some(maybe) = params.deleted_at {
        db::main::wallet::queries::set_deleted_at(params.id, maybe, pool).await?
    } else {
        wallet
    };
    Ok(wallet.into())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn update_wallet_changes_name() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "foo".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: wallet.id,
                name: Some("bar".into()),
                xpub: None,
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(res.name, "bar");
        assert_eq!(
            res.xpub,
            "xpub0000000000000000000000000000000000000000000000000000000000000000"
        );
        Ok(())
    }

    #[actix_web::test]
    async fn update_wallet_changes_xpub() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "foo".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: wallet.id,
                name: None,
                xpub: Some(
                    "xpub1111111111111111111111111111111111111111111111111111111111111111".into(),
                ),
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(
            res.xpub,
            "xpub1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(res.name, "foo");
        Ok(())
    }

    #[actix_web::test]
    async fn update_wallet_missing_returns_error() {
        let pool = pool();
        let res = super::run(
            super::Params {
                id: 999,
                name: Some("bar".into()),
                xpub: None,
                deleted_at: None,
            },
            &pool,
        )
        .await;
        assert!(res.is_err());
    }

    #[actix_web::test]
    async fn update_wallet_undeletes_when_deleted_at_is_null() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "foo".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        crate::db::main::wallet::queries::set_deleted_at(
            wallet.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: wallet.id,
                name: None,
                xpub: None,
                deleted_at: Some(None),
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn update_wallet_soft_deletes_when_deleted_at_is_timestamp() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "foo".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let when = time::OffsetDateTime::now_utc();
        let res = super::run(
            super::Params {
                id: wallet.id,
                name: None,
                xpub: None,
                deleted_at: Some(Some(when)),
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_some());
        Ok(())
    }

    #[actix_web::test]
    async fn update_wallet_does_not_touch_deleted_at_when_absent() -> Result<()> {
        let pool = pool();
        let wallet = crate::db::main::wallet::queries::insert(
            "foo".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: wallet.id,
                name: Some("bar".into()),
                xpub: None,
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_none());
        Ok(())
    }
}
