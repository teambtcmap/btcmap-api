use crate::{
    admin, conf::Conf, discord, element::Element, element_comment::ElementComment, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "add_element_comment";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
    pub comment: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<ElementComment> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<ElementComment> {
    let admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let element = Element::select_by_id_or_osm_id_async(params.id, &pool)
        .await?
        .ok_or("Element not found")?;
    let comment = ElementComment::insert_async(element.id, &params.comment, &pool).await?;
    let log_message = format!(
        "Admin {} added a comment to element {} ({}): {}",
        admin.name,
        element.name(),
        element.id,
        params.comment,
    );
    discord::post_message(&conf.discord_webhook_api, log_message).await;
    Ok(comment)
}
