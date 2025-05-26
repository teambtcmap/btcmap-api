use super::queries::Admin;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    name: impl Into<String>,
    password: impl Into<String>,
    pool: &Pool,
) -> Result<i64> {
    let name = name.into();
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| super::queries::insert(&name, &password, conn))
        .await?
}

pub async fn select_all(pool: &Pool) -> Result<Vec<Admin>> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_all(conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Admin> {
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_name(name: impl Into<String>, pool: &Pool) -> Result<Admin> {
    let name = name.into();
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_name(&name, conn))
        .await?
}

pub async fn set_password(id: i64, password: impl Into<String>, pool: &Pool) -> Result<()> {
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| super::queries::set_password(id, password, conn))
        .await?
}

pub async fn set_roles(admin_id: i64, roles: &[String], pool: &Pool) -> Result<()> {
    let roles = roles.to_vec();
    pool.get()
        .await?
        .interact(move |conn| super::queries::set_roles(admin_id, &roles, conn))
        .await?
}
