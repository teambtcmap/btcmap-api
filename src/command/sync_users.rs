use crate::user::User;
use crate::Connection;
use crate::Error;
use crate::Result;
use reqwest::StatusCode;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;
use tracing::error;
use tracing::info;

pub async fn run(conn: Connection) -> Result<()> {
    info!("Syncing users");

    let users = User::select_all(None, &conn)?;

    info!(users = users.len(), "Loaded all users from database");

    for cached_user in &users {
        let url = format!(
            "https://api.openstreetmap.org/api/0.6/user/{}.json",
            cached_user.id,
        );
        info!(url, "Querying OSM");

        let res = match reqwest::get(&url).await {
            Ok(v) => v,
            Err(e) => {
                error!(cached_user.id, ?e, "Failed to sync user");
                continue;
            }
        };

        if res.status() != StatusCode::OK {
            error!(
                cached_user.id,
                response_status = ?res.status(),
                "Failed to query a user",
            );
            continue;
        }

        let body = res.text().await?;
        let body: Value = serde_json::from_str(&body)?;
        let fresh_user: &Value = body.get("user").ok_or(Error::OsmApi(format!(
            "Failed to fetch user {}",
            cached_user.id
        )))?;

        let db_user_str = serde_json::to_string(&cached_user.osm_data)?;
        let fresh_user_str = serde_json::to_string(&fresh_user)?;

        if fresh_user_str != db_user_str {
            info!("Change detected");
            User::set_osm_data(
                cached_user.id,
                &serde_json::from_value(fresh_user.clone())?,
                &conn,
            )?;
        }

        // OSM admins suggested this timeout
        sleep(Duration::from_millis(5000)).await;
    }

    Ok(())
}
