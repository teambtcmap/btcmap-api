use crate::{admin, discord, element::Element, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use tracing::info;

const NAME: &str = "add_element_comment";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub comment: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<ElementComment> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
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
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(review)
}
