use super::OsmUser;
use crate::{osm, Error, Result};
use deadpool_sqlite::Pool;
use tracing::info;

pub async fn insert_user_if_not_exists(user_id: i64, pool: &Pool) -> Result<()> {
    if OsmUser::select_by_id_async(user_id, pool).await?.is_some() {
        info!(user_id, "User already exists");
        return Ok(());
    }
    match osm::api::get_user(user_id).await? {
        Some(user) => OsmUser::insert_async(user_id, user, pool).await?,
        None => Err(Error::OsmApi(format!(
            "User with id = {user_id} doesn't exist on OSM"
        )))?,
    };
    Ok(())
}
