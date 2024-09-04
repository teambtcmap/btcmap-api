use crate::{auth::Token, discord, element::Element, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub token: String,
    pub id: String,
    pub review: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<ElementComment> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
    let cloned_review = args.review.clone();
    let review = pool
        .get()
        .await?
        .interact(move |conn| ElementComment::insert(element.id, &cloned_review, conn))
        .await??;
    let log_message = format!(
        "{} added a comment to element {} ({}): {}",
        token.owner,
        element.name(),
        element.id,
        args.review,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(review)
}
