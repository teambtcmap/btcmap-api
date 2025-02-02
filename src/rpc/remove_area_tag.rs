use super::model::RpcArea;
use crate::{
    admin,
    area::{self},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
    pub tag: String,
}

pub const NAME: &str = "remove_area_tag";

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<RpcArea> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let area = area::service::remove_tag_async(params.id, &params.tag, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} removed tag {} from area {} ({})",
            admin.name,
            params.tag,
            area.name(),
            area.id,
        ),
    )
    .await;
    Ok(area.into())
}
