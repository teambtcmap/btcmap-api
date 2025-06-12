use crate::db;
use crate::{conf::Conf, discord, Result};
use argon2::PasswordVerifier;
use argon2::{Argon2, PasswordHash};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct Params {
    pub username: String,
    pub password: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct Res {
    pub token: String,
    pub time_ms: i128,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let error_cause_mask = "Invalid credentials";
    let start_time = OffsetDateTime::now_utc();
    let user = db::user::queries_async::select_by_name(params.username, &pool)
        .await
        .map_err(|_| error_cause_mask)?;
    let password_hash = PasswordHash::new(&user.password).map_err(|_| error_cause_mask)?;
    Argon2::default()
        .verify_password(params.password.as_bytes(), &password_hash)
        .map_err(|_| error_cause_mask)?;
    let token = Uuid::new_v4().to_string();
    db::access_token::queries_async::insert(user.id, &params.label, &token, &user.roles, &pool)
        .await?;
    let time_passed_ms = (OffsetDateTime::now_utc() - start_time).whole_milliseconds();
    let discord_message = format!(
        "User {} created a new API token labled {} ({time_passed_ms} ms)",
        user.name, params.label,
    );
    discord::post_message(&conf.discord_webhook_api, discord_message).await;
    Ok(Res {
        token,
        time_ms: time_passed_ms,
    })
}
