use crate::db;
use crate::{conf::Conf, db::admin::queries::Admin, discord, Result};
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
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub time_ms: i128,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let start_time = OffsetDateTime::now_utc();
    let old_password_hash = PasswordHash::new(&admin.password).unwrap();
    Argon2::default()
        .verify_password(params.old_password.as_bytes(), &old_password_hash)
        .unwrap();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.new_password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    db::admin::queries_async::set_password(admin.id, password_hash, pool).await?;
    let time_passed_ms = (OffsetDateTime::now_utc() - start_time).whole_milliseconds();
    let discord_message = format!(
        "Admin {} changed their password. It took {time_passed_ms} ms to process this call",
        admin.name,
    );
    discord::post_message(&conf.discord_webhook_api, discord_message).await;
    Ok(Res {
        time_ms: time_passed_ms,
    })
}
