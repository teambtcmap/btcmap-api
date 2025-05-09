use crate::{conf::Conf, db::admin::queries::Admin, discord, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub new_admin_name: String,
    pub new_admin_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let new_admin_id = crate::db::admin::queries_async::insert(
        params.new_admin_name,
        params.new_admin_password,
        pool,
    )
    .await?;
    let new_admin = crate::db::admin::queries_async::select_by_id(new_admin_id, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!("Admin {} added new admin {}", admin.name, new_admin.name),
    )
    .await;
    Ok(Res {
        name: new_admin.name,
        allowed_actions: new_admin.roles,
    })
}
