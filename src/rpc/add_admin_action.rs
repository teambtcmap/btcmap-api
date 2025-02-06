use crate::{admin::Admin, conf::Conf, discord, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub admin: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(params: Params, source_admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let target_admin = Admin::select_by_name_async(&params.admin, &pool).await?;
    let mut allowed_actions = target_admin.allowed_actions;
    if !allowed_actions.contains(&params.action) {
        allowed_actions.push(params.action.clone());
    }
    let target_admin =
        Admin::update_allowed_actions_async(target_admin.id, &allowed_actions, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} allowed action {} for admin {}",
            source_admin.name, params.action, target_admin.name
        ),
    )
    .await;
    Ok(Res {
        name: target_admin.name,
        allowed_actions: target_admin.allowed_actions,
    })
}
