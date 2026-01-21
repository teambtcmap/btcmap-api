use crate::db;
use crate::Result;
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
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub token: String,
    pub time_ms: i128,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let error_cause_mask = "Invalid credentials";
    let start_time = OffsetDateTime::now_utc();
    let user = db::user::queries::select_by_name(params.username, &pool)
        .await
        .map_err(|_| error_cause_mask)?;
    let password_hash = PasswordHash::new(&user.password).map_err(|_| error_cause_mask)?;
    Argon2::default()
        .verify_password(params.password.as_bytes(), &password_hash)
        .map_err(|_| error_cause_mask)?;
    let token = Uuid::new_v4().to_string();
    db::access_token::queries::insert(
        user.id,
        params.label.unwrap_or_default(),
        token.clone(),
        vec![],
        &pool,
    )
    .await?;
    let time_passed_ms = (OffsetDateTime::now_utc() - start_time).whole_milliseconds();
    Ok(Res {
        token,
        time_ms: time_passed_ms,
    })
}
