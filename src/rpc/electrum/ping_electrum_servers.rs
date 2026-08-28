use crate::{
    db::{self, main::electrum_server::schema::ElectrumServer},
    Result,
};
use deadpool_sqlite::Pool;
use electrum_client::{Client, ElectrumApi};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

const PING_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub include_deleted: Option<bool>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub priority: i64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<ElectrumServer> for Res {
    fn from(server: ElectrumServer) -> Self {
        Res {
            id: server.id,
            name: server.name,
            url: server.url,
            priority: server.priority,
            ok: false,
            latency_ms: None,
            error: None,
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

    let mut join_set: JoinSet<(i64, Result<Duration, String>)> = JoinSet::new();
    for server in &servers {
        let id = server.id;
        let url = server.url.clone();
        let spki_pin = server.spki_pin.clone();
        join_set.spawn_blocking(move || {
            let started = Instant::now();
            let res = ping_server(&url, &spki_pin);
            let elapsed = started.elapsed();
            (id, res.map(|()| elapsed).map_err(|e| e.to_string()))
        });
    }

    let mut results: Vec<Res> = servers.into_iter().map(Into::into).collect();
    while let Some(joined) = join_set.join_next().await {
        let (id, ping_res) = match joined {
            Ok(v) => v,
            Err(e) => {
                return Err(crate::Error::Other(format!("ping task join failed: {}", e)));
            }
        };
        let Some(slot) = results.iter_mut().find(|r| r.id == id) else {
            continue;
        };
        match ping_res {
            Ok(latency) => {
                slot.ok = true;
                slot.latency_ms = Some(latency.as_millis() as u64);
            }
            Err(err) => {
                slot.ok = false;
                slot.error = Some(err);
            }
        }
    }
    Ok(results)
}

fn ping_server(url: &str, spki_pin: &str) -> Result<()> {
    if spki_pin.is_empty() {
        let config = electrum_client::Config::builder()
            .timeout(Some(PING_TIMEOUT))
            .build();
        let client = Client::from_config(url, config)?;
        client.ping()?;
    } else {
        let mut client = crate::service::electrum_pinned::PinnedClient::connect(url, spki_pin)?;
        crate::service::electrum_pinned::PinnedClient::ping(&mut client)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::pool, Result};

    #[actix_web::test]
    async fn ping_electrum_servers_returns_one_entry_per_server() -> Result<()> {
        let pool = pool();
        crate::db::main::electrum_server::queries::insert(
            "a".into(),
            "tcp://127.0.0.1:1".into(),
            10,
            "".into(),
            &pool,
        )
        .await?;
        crate::db::main::electrum_server::queries::insert(
            "b".into(),
            "tcp://127.0.0.1:2".into(),
            0,
            "".into(),
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
        assert_eq!(res.len(), 2);
        for entry in &res {
            assert!(!entry.ok);
            assert!(entry.error.is_some());
            assert_eq!(entry.latency_ms, None);
        }
        Ok(())
    }

    #[actix_web::test]
    async fn ping_electrum_servers_hides_soft_deleted_by_default() -> Result<()> {
        let pool = pool();
        let a = crate::db::main::electrum_server::queries::insert(
            "a".into(),
            "tcp://127.0.0.1:1".into(),
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
        crate::db::main::electrum_server::queries::insert(
            "b".into(),
            "tcp://127.0.0.1:2".into(),
            0,
            "".into(),
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
