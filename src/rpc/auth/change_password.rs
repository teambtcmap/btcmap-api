use crate::db;
use crate::service::auth::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};
use crate::Result;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub changed: bool,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    if params.new_password.len() < MIN_PASSWORD_LENGTH {
        return Err(
            format!("New password is too short, use at least {MIN_PASSWORD_LENGTH} chars").into(),
        );
    }
    if params.new_password.len() > MAX_PASSWORD_LENGTH {
        return Err(
            format!("New password is too long, use at most {MAX_PASSWORD_LENGTH} chars").into(),
        );
    }
    let err_invalid_username_password = "Incorrect username or password";
    let err_generic = "Unexpected error, please contact administrator";
    let user = db::main::user::queries::select_by_name(params.username, pool)
        .await
        .map_err(|_| err_invalid_username_password)?;
    let old_password_hash = PasswordHash::new(&user.password).map_err(|_| err_generic)?;
    Argon2::default()
        .verify_password(params.old_password.as_bytes(), &old_password_hash)
        .map_err(|_| err_invalid_username_password)?;
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.new_password.as_bytes(), &salt)
        .map_err(|_| err_generic)?
        .to_string();
    db::main::user::queries::set_password(user.id, password_hash, pool)
        .await
        .map_err(|_| err_generic)?;
    Ok(Res { changed: true })
}
