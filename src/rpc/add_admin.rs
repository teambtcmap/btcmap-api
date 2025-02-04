use crate::{
    admin::{service::check_rpc, Admin},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

pub const NAME: &str = "add_admin";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub new_admin_name: String,
    pub new_admin_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let admin = check_rpc(params.password, NAME, &pool).await?;
    let new_admin =
        Admin::insert_async(params.new_admin_name, params.new_admin_password, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!("Admin {} added new admin {}", admin.name, new_admin.name),
    )
    .await;
    Ok(Res {
        name: new_admin.name,
        allowed_actions: new_admin.allowed_actions,
    })
}
