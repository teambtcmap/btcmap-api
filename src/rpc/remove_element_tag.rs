use crate::conf::Conf;
use crate::discord;
use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "remove_element_tag";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub element_id: String,
    pub tag_name: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<Element> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Element> {
    let admin = admin::service::check_rpc(params.password, NAME, pool).await?;
    let element = Element::select_by_id_or_osm_id_async(params.element_id, pool)
        .await?
        .ok_or("Element not found")?;
    let element = Element::remove_tag_async(element.id, &params.tag_name, pool).await?;
    let log_message = format!(
        "Admin {} removed tag {} from element {} https://api.btcmap.org/v4/elements/{}",
        admin.name,
        params.tag_name,
        element.name(),
        element.id,
    );
    discord::post_message(&conf.discord_webhook_api, log_message).await;
    Ok(element)
}
