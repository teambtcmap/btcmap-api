use crate::{
    admin::{self, Admin},
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
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

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<Res> {
    let source_admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let cloned_args_admin = args.admin.clone();
    let target_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_name(&cloned_args_admin, conn))
        .await??
        .ok_or(format!(
            "Admin user with name = {} does not exist",
            args.admin,
        ))?;
    let mut allowed_actions = target_admin.allowed_actions;
    if !allowed_actions.contains(&args.action) {
        allowed_actions.push(args.action.clone());
    }
    let target_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::set_allowed_actions(target_admin.id, &allowed_actions, conn))
        .await??
        .ok_or(format!(
            "Failed to update allowed actions for admin user {}",
            args.admin,
        ))?;
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
