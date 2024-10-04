use crate::{admin::Admin, discord, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub admin_name: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let source_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let target_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_name(&args.admin_name, conn))
        .await??
        .unwrap();
    let mut allowed_actions = target_admin.allowed_actions;
    if !allowed_actions.contains(&args.action) {
        allowed_actions.push(args.action.clone());
    }
    let target_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::set_allowed_actions(target_admin.id, &allowed_actions, conn))
        .await??
        .unwrap();
    let log_message = format!(
        "{} allowed action '{}' for admin {}",
        source_admin.name, args.action, target_admin.name,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        name: target_admin.name,
        allowed_actions: target_admin.allowed_actions,
    })
}
