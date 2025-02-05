use super::model::RpcArea;
use crate::{
    admin::Admin,
    area::{self},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub tag: String,
}

pub async fn run_internal(
    params: Params,
    admin: &Admin,
    pool: &Pool,
    conf: &Conf,
) -> Result<RpcArea> {
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
