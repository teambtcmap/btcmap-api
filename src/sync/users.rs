use crate::osm::osm;
use crate::user::User;
use crate::Connection;
use crate::Result;
use serde_json::Value;
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::sleep;
use tokio::time::Duration;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn run(conn: Connection) -> Result<()> {
    let users = User::select_all(None, &conn)?;

    for (i, user) in users.iter().enumerate() {
        info!("Syncing users ({} of {})", i + 1, users.len());

        if user.tags.get("osm:missing") == Some(&Value::Bool(true)) {
            info!(user.osm_data.id, "This user is missing from OSM, skipping");
            continue;
        }

        let yesterday = OffsetDateTime::now_utc()
            .add(time::Duration::days(-1))
            .format(&Rfc3339)?;

        if user.tags.contains_key("osm:sync:date") {
            let last_sync_date = user.tags["osm:sync:date"].as_str().unwrap();
            if last_sync_date > yesterday.as_str() {
                info!(user.osm_data.id, "Last sync date is fresh enough, skipping");
                continue;
            }
        }

        match osm::get_user(user.osm_data.id).await {
            Ok(new_osm_data) => match new_osm_data {
                Some(new_osm_data) => {
                    if new_osm_data != user.osm_data {
                        info!(
                            old_osm_data = serde_json::to_string(&user.osm_data)?,
                            new_osm_data = serde_json::to_string(&new_osm_data)?,
                            "Change detected",
                        );
                        User::set_osm_data(user.id, &new_osm_data, &conn)?;
                    } else {
                        info!("No changes detected")
                    }

                    let now = OffsetDateTime::now_utc();
                    let now: String = now.format(&Rfc3339)?;
                    user.set_tag("osm:sync:date", &Value::String(now), &conn)?;
                }
                None => {
                    warn!(user.osm_data.id, "User no longer exists on OSM");
                    user.set_tag("osm:missing", &Value::Bool(true), &conn)?;
                }
            },
            Err(e) => error!("Failed to fetch user {} {}", user.osm_data.id, e),
        }

        // OSM admins suggested this timeout
        sleep(Duration::from_millis(5000)).await;
    }

    Ok(())
}
