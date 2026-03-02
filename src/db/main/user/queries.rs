use super::{blocking_queries, schema::Role, schema::User};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    name: impl Into<String>,
    password: impl Into<String>,
    pool: &Pool,
) -> Result<User> {
    let name = name.into();
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&name, &password, conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<User> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_name(name: impl Into<String>, pool: &Pool) -> Result<User> {
    let name = name.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_name(&name, conn))
        .await?
}

pub async fn set_password(id: i64, password: impl Into<String>, pool: &Pool) -> Result<()> {
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_password(id, password, conn))
        .await?
}

pub async fn set_roles(admin_id: i64, roles: &[Role], pool: &Pool) -> Result<User> {
    let roles = roles.to_vec();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_roles(admin_id, &roles, conn))
        .await?
}
