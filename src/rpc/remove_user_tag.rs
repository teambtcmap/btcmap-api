use crate::{admin, discord, user::User, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "remove_user_tag";

#[derive(Deserialize, Clone)]
pub struct Args {
    pub password: String,
    pub user_name: String,
    pub tag_name: String,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let cloned_args_user_name = args.user_name.clone();
    let cloned_args_tag_name = args.tag_name.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::select_by_name(&cloned_args_user_name, conn))
        .await??
        .ok_or(format!("There is no user with name = {}", args.user_name))?;
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::remove_tag(user.id, &cloned_args_tag_name, conn))
        .await??
        .ok_or(format!("There is no user with name = {}", args.user_name))?;
    let log_message = format!(
        "{} removed tag {} for user {} https://api.btcmap.org/v3/users/{}",
        admin.name, args.tag_name, args.user_name, user.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        id: user.id,
        tags: user.tags,
    })
}
