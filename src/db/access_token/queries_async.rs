use super::queries::AccessToken;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    user_id: i64,
    name: impl Into<String>,
    secret: impl Into<String>,
    roles: &[String],
    pool: &Pool,
) -> Result<i64> {
    let name = name.into();
    let secret = secret.into();
    let roles = roles.to_vec();
    pool.get()
        .await?
        .interact(move |conn| super::queries::insert(user_id, &name, &secret, &roles, conn))
        .await?
}

pub async fn select_by_secret(secret: impl Into<String>, pool: &Pool) -> Result<AccessToken> {
    let secret = secret.into();
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_secret(&secret, conn))
        .await?
}
