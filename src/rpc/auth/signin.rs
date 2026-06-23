use crate::db;
use crate::Result;
use argon2::PasswordVerifier;
use argon2::{Argon2, PasswordHash};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct Params {
    pub username: String,
    pub password: String,
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub api_key: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let error_cause_mask = "Invalid credentials";
    let user = db::main::user::queries::select_by_name(params.username, pool)
        .await
        .map_err(|_| error_cause_mask)?;
    let password_hash = PasswordHash::new(&user.password).map_err(|_| error_cause_mask)?;
    Argon2::default()
        .verify_password(params.password.as_bytes(), &password_hash)
        .map_err(|_| error_cause_mask)?;
    let api_key = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(
        user.id,
        params.label.unwrap_or_default(),
        api_key.clone(),
        vec![],
        pool,
    )
    .await?;
    Ok(Res { api_key })
}
