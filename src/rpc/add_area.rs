use super::model::RpcArea;
use crate::admin;
use crate::conf::Conf;
use crate::Result;
use crate::{area, discord};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::{Map, Value};

pub const NAME: &str = "add_area";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub tags: Map<String, Value>,
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let area = area::service::insert_async(params.tags, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} created area {} ({})",
            admin.name,
            area.name(),
            area.id,
        ),
    )
    .await;
    Ok(area.into())
}
