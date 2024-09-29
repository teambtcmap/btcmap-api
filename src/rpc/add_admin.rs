use crate::{auth::Token, discord, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Args {
    pub token: String,
    pub new_admin_name: String,
    pub new_admin_token: String,
    pub new_admin_allowed_methods: Vec<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub token: String,
    pub allowed_methods: Vec<String>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let new_token = pool
        .get()
        .await?
        .interact(move |conn| {
            Token::insert(
                &args.new_admin_name,
                &Uuid::new_v4().to_string(),
                args.new_admin_allowed_methods,
                conn,
            )
        })
        .await??
        .unwrap();
    let log_message = format!(
        "{} added new admin user {} with the following allowed methods: {}",
        token.owner,
        new_token.owner,
        serde_json::to_string(&new_token.allowed_methods)?,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        name: new_token.owner,
        token: new_token.secret,
        allowed_methods: new_token.allowed_methods,
    })
}
