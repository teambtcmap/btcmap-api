use crate::{
    db::{self, main::electrum_server::schema::ElectrumServer},
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
    pub url: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub spki_pin: Option<String>,
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
pub struct Res {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub priority: i64,
    pub spki_pin: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<ElectrumServer> for Res {
    fn from(server: ElectrumServer) -> Self {
        Res {
            id: server.id,
            name: server.name,
            url: server.url,
            priority: server.priority,
            spki_pin: server.spki_pin,
            created_at: server.created_at,
            updated_at: server.updated_at,
            deleted_at: server.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let server = db::main::electrum_server::queries::update(
        params.id,
        params.name,
        params.url,
        params.priority,
        params.spki_pin,
        pool,
    )
    .await?;
    let server = if let Some(maybe) = params.deleted_at {
        db::main::electrum_server::queries::set_deleted_at(params.id, maybe, pool).await?
    } else {
        server
    };
    Ok(server.into())
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::pool, Result};

    #[actix_web::test]
    async fn update_electrum_server_changes_priority() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            5,
            "".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: server.id,
                name: None,
                url: None,
                priority: Some(99),
                spki_pin: None,
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(res.priority, 99);
        assert_eq!(res.name, "foo");
        assert_eq!(res.url, "ssl://foo:50002");
        Ok(())
    }

    #[actix_web::test]
    async fn update_electrum_server_clears_spki_pin() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            0,
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: server.id,
                name: None,
                url: None,
                priority: None,
                spki_pin: Some(String::new()),
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert_eq!(res.spki_pin, "");
        Ok(())
    }

    #[actix_web::test]
    async fn update_electrum_server_missing_returns_error() {
        let pool = pool();
        let res = super::run(
            super::Params {
                id: 999,
                name: Some("bar".into()),
                url: None,
                priority: None,
                spki_pin: None,
                deleted_at: None,
            },
            &pool,
        )
        .await;
        assert!(res.is_err());
    }

    #[actix_web::test]
    async fn update_electrum_server_undeletes_when_deleted_at_is_null() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        crate::db::main::electrum_server::queries::set_deleted_at(
            server.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: server.id,
                name: None,
                url: None,
                priority: None,
                spki_pin: None,
                deleted_at: Some(None),
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn update_electrum_server_soft_deletes_when_deleted_at_is_timestamp() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        let when = time::OffsetDateTime::now_utc();
        let res = super::run(
            super::Params {
                id: server.id,
                name: None,
                url: None,
                priority: None,
                spki_pin: None,
                deleted_at: Some(Some(when)),
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_some());
        Ok(())
    }

    #[actix_web::test]
    async fn update_electrum_server_does_not_touch_deleted_at_when_absent() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        let res = super::run(
            super::Params {
                id: server.id,
                name: Some("bar".into()),
                url: None,
                priority: None,
                spki_pin: None,
                deleted_at: None,
            },
            &pool,
        )
        .await?;
        assert!(res.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn deserialize_optional_field_distinguishes_absent_from_null() {
        use serde::Deserialize;
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default, deserialize_with = "super::deserialize_optional_field")]
            v: Option<Option<i64>>,
        }
        let absent: Wrapper = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.v, None);
        let explicit_null: Wrapper = serde_json::from_str(r#"{"v":null}"#).unwrap();
        assert_eq!(explicit_null.v, Some(None));
        let value: Wrapper = serde_json::from_str(r#"{"v":42}"#).unwrap();
        assert_eq!(value.v, Some(Some(42)));
    }
}
