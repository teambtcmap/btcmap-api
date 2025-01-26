use crate::{
    admin, conf::Conf, discord, element::Element, element_comment::ElementComment, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "add_element_comment";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub comment: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<ElementComment> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let cloned_id = args.id.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&cloned_id, conn))
        .await??
        .ok_or(format!("There is no element with id = {}", &args.id))?;
    let cloned_comment = args.comment.clone();
    let review = pool
        .get()
        .await?
        .interact(move |conn| ElementComment::insert(element.id, &cloned_comment, conn))
        .await??;
    let log_message = format!(
        "{} added a comment to element {} ({}): {}",
        admin.name,
        element.name(),
        element.id,
        args.comment,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(review)
}
