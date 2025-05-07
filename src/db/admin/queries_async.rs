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

pub async fn select_by_password(password: impl Into<String>, pool: &Pool) -> Result<Admin> {
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| super::queries::select_by_password(&password, conn))
        .await?
}

pub async fn update_allowed_actions(
    admin_id: i64,
    new_allowed_actions: &[String],
    pool: &Pool,
) -> Result<()> {
    let new_allowed_actions = new_allowed_actions.to_vec();
    pool.get()
        .await?
        .interact(move |conn| {
            super::queries::update_allowed_actions(admin_id, &new_allowed_actions, conn)
        })
        .await?
}
