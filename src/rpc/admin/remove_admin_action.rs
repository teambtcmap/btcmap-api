use crate::{conf::Conf, db::admin::queries::Admin, discord, Result};
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
    let target_admin = crate::db::admin::queries_async::select_by_name(&params.admin, pool).await?;
    let allowed_actions: Vec<String> = target_admin
        .allowed_actions
        .into_iter()
        .filter(|it| it != &params.action)
        .collect();
    crate::db::admin::queries_async::update_allowed_actions(
        target_admin.id,
        &allowed_actions,
        pool,
    )
    .await?;
    let target_admin = crate::db::admin::queries_async::select_by_id(target_admin.id, pool).await?;
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
