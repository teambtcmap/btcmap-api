use crate::{
    db::{self, main::electrum_server::schema::ElectrumServer},
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

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    let include_deleted = params.include_deleted.unwrap_or(false);
    let servers = db::main::electrum_server::queries::select_all(pool).await?;
    let servers: Vec<ElectrumServer> = servers
        .into_iter()
        .filter(|it| include_deleted || it.deleted_at.is_none())
        .collect();
    Ok(servers.into_iter().map(Into::into).collect())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;

    #[actix_web::test]
    async fn get_electrum_servers_returns_active_only_by_default() -> Result<()> {
        let pool = pool();
        let a = crate::db::main::electrum_server::queries::insert(
            "a".into(),
            "ssl://a:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        let _b = crate::db::main::electrum_server::queries::insert(
            "b".into(),
            "ssl://b:50002".into(),
            0,
            "".into(),
            &pool,
        )
        .await?;
        crate::db::main::electrum_server::queries::set_deleted_at(
            a.id,
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
        assert_eq!(res[0].name, "b");
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
}
