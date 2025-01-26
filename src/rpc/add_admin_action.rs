use crate::{
    admin::{self, Admin},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "add_admin_action";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub admin: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let source_admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let target_admin = Admin::select_by_name_async(&args.admin, &pool).await?;
    let mut allowed_actions = target_admin.allowed_actions;
    if !allowed_actions.contains(&args.action) {
        allowed_actions.push(args.action.clone());
    }
    let target_admin =
        Admin::update_allowed_actions_async(target_admin.id, &allowed_actions, &pool).await?;
    let log_message = format!(
        "Admin {} allowed action '{}' for admin {}",
        source_admin.name, args.action, target_admin.name,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(Res {
        name: target_admin.name,
        allowed_actions: target_admin.allowed_actions,
    })
}
