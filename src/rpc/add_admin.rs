use crate::{
    admin::{service::check_rpc, Admin},
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "add_admin";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub new_admin_name: String,
    pub new_admin_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = check_rpc(args.password, NAME, &pool).await?;
    let new_admin =
        Admin::insert_async(args.new_admin_name, args.new_admin_password, &pool).await?;
    let log_message = format!(
        "{} added new admin user {} with the following allowed actions: {}",
        admin.name,
        new_admin.name,
        serde_json::to_string(&new_admin.allowed_actions)?,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        name: new_admin.name,
        allowed_actions: new_admin.allowed_actions,
    })
}
