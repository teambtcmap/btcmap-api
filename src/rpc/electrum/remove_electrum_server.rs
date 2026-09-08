use crate::{
    db::{self, main::electrum_server::schema::ElectrumServer},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
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
    let server = db::main::electrum_server::queries::set_deleted_at(
        params.id,
        Some(OffsetDateTime::now_utc()),
        pool,
    )
    .await?;
    Ok(server.into())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn remove_electrum_server_soft_deletes() -> Result<()> {
        let pool = pool();
        let server = crate::db::main::electrum_server::queries::insert(
            "foo".into(),
            "ssl://foo:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        let res = super::run(super::Params { id: server.id }, &pool).await?;
        assert!(res.deleted_at.is_some());
        Ok(())
    }

    #[actix_web::test]
    async fn remove_electrum_server_missing_returns_error() {
        let pool = pool();
        let res = super::run(super::Params { id: 999 }, &pool).await;
        assert!(res.is_err());
    }
}
