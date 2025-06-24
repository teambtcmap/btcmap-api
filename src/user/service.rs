use crate::{db, osm, Error, Result};
use deadpool_sqlite::Pool;
use tracing::info;

pub async fn insert_user_if_not_exists(user_id: i64, pool: &Pool) -> Result<()> {
    if db::osm_user::queries_async::select_by_id(user_id, pool)
        .await
        .is_ok()
    {
        info!(user_id, "User already exists");
        return Ok(());
    }
    match osm::api::get_user(user_id).await? {
        Some(user) => db::osm_user::queries_async::insert(user_id, user, pool).await?,
        None => Err(Error::OsmApi(format!(
            "User with id = {user_id} doesn't exist on OSM"
        )))?,
    };
    Ok(())
}
