use crate::Result;
use argon2::Argon2;
use argon2::PasswordHasher;

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
