use crate::{
    conf::Conf,
    db::{self, admin::queries::Admin},
    discord, Result,
};
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
    let target_admin = db::admin::queries_async::select_by_name(&params.admin, pool).await?;
    let mut roles = target_admin.roles;
    if !roles.contains(&params.action) {
        roles.push(params.action.clone());
    }
    db::admin::queries_async::set_roles(target_admin.id, &roles, pool).await?;
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
        allowed_actions: roles,
    })
}
