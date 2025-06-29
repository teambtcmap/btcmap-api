use crate::conf::Conf;
use crate::db;
use crate::event::Event;
use crate::osm;
use crate::service::discord;
use crate::user;
use crate::Result;
use deadpool_sqlite::Pool;
use serde_json::Value;
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn on_new_event(event: &Event, pool: &Pool) -> Result<()> {
    user::service::insert_user_if_not_exists(event.user_id, pool).await?;
    let user = db::osm_user::queries_async::select_by_id(event.user_id, pool).await?;

    let message = match event.r#type.as_str() {
        "create" => format!(
            "{} added https://www.openstreetmap.org/{}/{}",
            user.osm_data.display_name, event.element_osm_type, event.element_osm_id
        ),
        "update" => format!(
            "{} updated https://www.openstreetmap.org/{}/{}",
            user.osm_data.display_name, event.element_osm_type, event.element_osm_id
        ),
        "delete" => format!(
            "{} removed https://www.openstreetmap.org/{}/{}",
            user.osm_data.display_name, event.element_osm_type, event.element_osm_id
        ),
        _ => "".into(),
    };
    info!(message);
    let conf = Conf::select_async(pool).await?;
    discord::send(message, discord::Channel::OsmChanges, &conf);

    if user.tags.get("osm:missing") == Some(&Value::Bool(true)) {
        info!(user.osm_data.id, "This user is missing from OSM, skipping");
        return Ok(());
    }

    let hour_ago = OffsetDateTime::now_utc()
        .add(time::Duration::hours(-1))
        .format(&Rfc3339)?;

    if user.tags.contains_key("osm:sync:date") {
        let last_sync_date = user.tags["osm:sync:date"].as_str().unwrap();
        if last_sync_date > hour_ago.as_str() {
            info!(
                event.user_id,
                last_sync_date, "Last sync date is fresh enough, skipping"
            );
            return Ok(());
        }
    }

    match osm::api::get_user(user.osm_data.id).await {
        Ok(new_osm_data) => match new_osm_data {
            Some(new_osm_data) => {
                if new_osm_data != user.osm_data {
                    info!(
                        old_osm_data = serde_json::to_string(&user.osm_data)?,
                        new_osm_data = serde_json::to_string(&new_osm_data)?,
                        "User data changed",
                    );
                    db::osm_user::queries_async::set_osm_data(user.id, new_osm_data, pool).await?;
                } else {
                    info!("User data didn't change")
                }

                let now = OffsetDateTime::now_utc();
                let now: String = now.format(&Rfc3339)?;
                db::osm_user::queries_async::set_tag(
                    user.id,
                    "osm:sync:date".into(),
                    Value::String(now),
                    pool,
                )
                .await?;
            }
            None => {
                warn!(user.osm_data.id, "User no longer exists on OSM");
                db::osm_user::queries_async::set_tag(
                    user.id,
                    "osm:missing".into(),
                    Value::Bool(true),
                    pool,
                )
                .await?;
            }
        },
        Err(e) => error!("Failed to fetch user {} {}", user.osm_data.id, e),
    }

    Ok(())
}
