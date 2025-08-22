use crate::{db, service, Result};
use deadpool_sqlite::Pool;
use tracing::info;

pub async fn insert_user_if_not_exists(user_id: i64, pool: &Pool) -> Result<()> {
    if db::osm_user::queries::select_by_id(user_id, pool)
        .await
        .is_ok()
    {
        info!(user_id, "User already exists");
        return Ok(());
    }
    match service::osm::get_user(user_id).await? {
        Some(user) => db::osm_user::queries::insert(user_id, user, pool).await?,
        None => Err(format!("User with id = {user_id} doesn't exist on OSM"))?,
    };
    Ok(())
}
