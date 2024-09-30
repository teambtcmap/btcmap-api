use crate::{admin::Admin, discord, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Args {
    pub password: String,
    pub new_admin_name: String,
    pub new_admin_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub password: String,
    pub allowed_methods: Vec<String>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let new_admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::insert(&args.new_admin_name, &args.new_admin_password, conn))
        .await??
        .unwrap();
    let log_message = format!(
        "{} added new admin user {} with the following allowed methods: {}",
        admin.name,
        new_admin.name,
        serde_json::to_string(&new_admin.allowed_methods)?,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        name: new_admin.name,
        password: new_admin.password,
        allowed_methods: new_admin.allowed_methods,
    })
}
