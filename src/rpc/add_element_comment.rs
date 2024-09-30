use crate::{admin::Admin, discord, element::Element, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub comment: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<ElementComment> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
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
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(review)
}
