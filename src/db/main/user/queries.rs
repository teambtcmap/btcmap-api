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

#[allow(dead_code)]
pub async fn select_by_npub(npub: impl Into<String>, pool: &Pool) -> Result<Option<User>> {
    let npub = npub.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_npub(&npub, conn))
        .await?
}

pub async fn set_password(id: i64, password: impl Into<String>, pool: &Pool) -> Result<usize> {
    let password = password.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_password(id, password, conn))
        .await?
}

pub async fn set_name(id: i64, name: impl Into<String>, pool: &Pool) -> Result<User> {
    let name = name.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_name(id, &name, conn))
        .await?
}

pub async fn set_roles(admin_id: i64, roles: &[Role], pool: &Pool) -> Result<User> {
    let roles = roles.to_vec();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_roles(admin_id, &roles, conn))
        .await?
}

#[allow(dead_code)]
pub async fn set_saved_places(id: i64, saved_places: &[i64], pool: &Pool) -> Result<User> {
    let saved_places = saved_places.to_vec();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_saved_places(id, &saved_places, conn))
        .await?
}

#[allow(dead_code)]
pub async fn set_saved_areas(id: i64, saved_areas: &[i64], pool: &Pool) -> Result<User> {
    let saved_areas = saved_areas.to_vec();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_saved_areas(id, &saved_areas, conn))
        .await?
}

#[allow(dead_code)]
pub async fn set_npub(id: i64, npub: Option<String>, pool: &Pool) -> Result<User> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_npub(id, npub, conn))
        .await?
}
