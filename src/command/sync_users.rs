use crate::model::user;
use crate::model::User;
use crate::Connection;
use crate::Error;
use crate::Result;
use reqwest::StatusCode;
use rusqlite::named_params;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

pub async fn run(db: Connection) -> Result<()> {
    log::info!("Syncing users");

    let users: Vec<User> = db
        .prepare(user::SELECT_ALL)?
        .query_map([], user::SELECT_ALL_MAPPER)?
        .collect::<Result<_, _>>()?;

    log::info!("Found {} cached users", users.len());

    for cached_user in &users {
        let url = format!(
            "https://api.openstreetmap.org/api/0.6/user/{}.json",
            cached_user.id,
        );
        log::info!("Querying {url}");

        let res = match reqwest::get(&url).await {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to sync user {}: {:?}", cached_user.id, e);
                continue;
            }
        };

        if res.status() != StatusCode::OK {
            log::error!(
                "Failed to query a user with id {}, response status: {:?}",
                cached_user.id,
                res.status(),
            );
            continue;
        }

        let body = res.text().await?;
        let body: Value = serde_json::from_str(&body)?;
        let fresh_user: &Value = body.get("user").ok_or(Error::Other(format!(
            "Failed to fetch user {}",
            cached_user.id
        )))?;

        let db_user_str = serde_json::to_string(&cached_user.osm_json)?;
        let fresh_user_str = serde_json::to_string(&fresh_user)?;

        if fresh_user_str != db_user_str {
            log::info!("Change detected");

            db.execute(
                user::UPDATE_OSM_JSON,
                named_params! {
                    ":id": cached_user.id,
                    ":osm_json": fresh_user_str,
                },
            )?;
        }

        sleep(Duration::from_millis(5000)).await;
    }

    Ok(())
}
