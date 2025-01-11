use super::User;
use crate::{osm, Error, Result};
use rusqlite::Connection;
use tracing::info;

pub async fn insert_user_if_not_exists(user_id: i64, conn: &Connection) -> Result<()> {
    if User::select_by_id(user_id, conn)?.is_some() {
        info!(user_id, "User already exists");
        return Ok(());
    }
    match osm::api::get_user(user_id).await? {
        Some(user) => User::insert(user_id, &user, conn)?,
        None => Err(Error::OsmApi(format!(
            "User with id = {user_id} doesn't exist on OSM"
        )))?,
    };
    Ok(())
}
