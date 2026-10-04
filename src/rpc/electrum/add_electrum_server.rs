use crate::{
    db::{self, main::electrum_server::schema::ElectrumServer},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub spki_pin: Option<String>,
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
    let priority = params.priority.unwrap_or(0);
    let spki_pin = params.spki_pin.unwrap_or_default();
    let server = db::main::electrum_server::queries::insert(
        params.name,
        params.url,
        priority,
        spki_pin,
        pool,
    )
    .await?;
    Ok(server.into())
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::pool, Result};

    #[actix_web::test]
    async fn add_electrum_server() -> Result<()> {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            url: "ssl://foo:50002".into(),
            priority: Some(10),
            spki_pin: None,
        };
        let res = super::run(params, &pool).await?;
        assert_eq!(res.name, "foo");
        assert_eq!(res.url, "ssl://foo:50002");
        assert_eq!(res.priority, 10);
        assert_eq!(res.spki_pin, "");
        Ok(())
    }

    #[actix_web::test]
    async fn add_non_ssl_electrum_server() -> Result<()> {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            url: "tcp://foo:50001".into(),
            priority: Some(10),
            spki_pin: None,
        };
        let res = super::run(params, &pool).await?;
        assert_eq!(res.url, "tcp://foo:50001");
        assert_eq!(res.spki_pin, "");
        Ok(())
    }

    #[actix_web::test]
    async fn add_electrum_server_with_pin() -> Result<()> {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            url: "ssl://foo:50002".into(),
            priority: None,
            spki_pin: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
        };
        let res = super::run(params, &pool).await?;
        assert!(res.spki_pin.starts_with("sha256:"));
        Ok(())
    }

    #[actix_web::test]
    async fn add_electrum_server_default_priority() -> Result<()> {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            url: "ssl://foo:50002".into(),
            priority: None,
            spki_pin: None,
        };
        let res = super::run(params, &pool).await?;
        assert_eq!(res.priority, 0);
        Ok(())
    }

    #[actix_web::test]
    async fn add_electrum_server_rejects_duplicate_url() {
        let pool = pool();
        let params = super::Params {
            name: "foo".into(),
            url: "ssl://foo:50002".into(),
            priority: None,
            spki_pin: None,
        };
        super::run(params, &pool).await.unwrap();
        let params = super::Params {
            name: "bar".into(),
            url: "ssl://foo:50002".into(),
            priority: None,
            spki_pin: None,
        };
        let res = super::run(params, &pool).await;
        assert!(res.is_err());
    }
}
