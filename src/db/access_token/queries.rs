use super::{
    blocking_queries,
    schema::{AccessToken, Role},
};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    user_id: i64,
    name: String,
    secret: String,
    roles: Vec<Role>,
    pool: &Pool,
) -> Result<AccessToken> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(user_id, &name, &secret, &roles, conn))
        .await?
}

pub async fn select_by_secret(secret: String, pool: &Pool) -> Result<AccessToken> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_secret(&secret, conn))
        .await?
}
