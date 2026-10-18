use crate::Result;
use argon2::Argon2;
use argon2::PasswordHasher;
use uuid::Uuid;

pub const MIN_PASSWORD_LENGTH: usize = 12;
pub const MAX_PASSWORD_LENGTH: usize = 64;

/// Argon2 hash of `password`, in PHC string format, as stored in
/// `user.password`.
pub fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string().into())
}

/// A password nobody knows, hashed: every account has one, so an empty
/// password can never stand in for a credential.
pub fn random_password_hash() -> Result<String> {
    hash_password(&Uuid::new_v4().to_string())
}
