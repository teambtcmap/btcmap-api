use crate::db;
use crate::{conf::Conf, discord, Result};
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize, Clone)]
pub struct Params {
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub time_ms: i128,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let error_cause_mask = "Something went wrong, please contact administrator";
    let start_time = OffsetDateTime::now_utc();
    let user = db::admin::queries_async::select_by_name(params.username, pool)
        .await
        .map_err(|_| error_cause_mask)?;
    let old_password_hash = PasswordHash::new(&user.password).map_err(|_| error_cause_mask)?;
    Argon2::default()
        .verify_password(params.old_password.as_bytes(), &old_password_hash)
        .map_err(|_| error_cause_mask)?;
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.new_password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    db::admin::queries_async::set_password(user.id, password_hash, pool).await?;
    let time_passed_ms = (OffsetDateTime::now_utc() - start_time).whole_milliseconds();
    let discord_message = format!(
        "User {} changed their password ({time_passed_ms} ms)",
        user.name,
    );
    discord::post_message(&conf.discord_webhook_api, discord_message).await;
    Ok(Res {
        time_ms: time_passed_ms,
    })
}
