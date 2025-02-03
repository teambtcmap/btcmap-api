use crate::{
    admin::{self, Admin},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "remove_admin_action";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub admin: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<Res> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let source_admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let target_admin = Admin::select_by_name_async(&params.admin, &pool).await?;
    let allowed_actions: Vec<String> = target_admin
        .allowed_actions
        .into_iter()
        .filter(|it| it != &params.action)
        .collect();
    let target_admin =
        Admin::update_allowed_actions_async(target_admin.id, &allowed_actions, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} removed action {} for admin {}",
            source_admin.name, params.action, target_admin.name
        ),
    )
    .await;
    Ok(Res {
        name: target_admin.name,
        allowed_actions: target_admin.allowed_actions,
    })
}
