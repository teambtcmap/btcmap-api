use crate::db;
use crate::db::main::element_event::schema::ElementEvent;
use crate::service;
use crate::service::matrix;
use crate::service::matrix::ROOM_OSM_CHANGES;
use crate::Result;
use deadpool_sqlite::Pool;
use matrix_sdk::Client;
use serde_json::Value;
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn on_new_event(
    event: &ElementEvent,
    pool: &Pool,
    matrix_client: &Option<Client>,
) -> Result<()> {
    service::user::insert_user_if_not_exists(event.user_id, pool).await?;
    let user = db::main::osm_user::queries::select_by_id(event.user_id, pool).await?;
    let element = db::main::element::queries::select_by_id(event.element_id, pool).await?;

    let message = match event.r#type.as_str() {
        "create" => format!(
            "{} added https://btcmap.org/merchant/{}",
            user.osm_data.display_name, element.id,
        ),
        "update" => format!(
            "{} updated https://btcmap.org/merchant/{}",
            user.osm_data.display_name, element.id,
        ),
        "delete" => format!(
            "{} removed https://btcmap.org/merchant/{}",
            user.osm_data.display_name, element.id,
        ),
        _ => "".into(),
    };
    info!(message);
    matrix::send_message(matrix_client, ROOM_OSM_CHANGES, &message);

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

    match service::osm::get_user(user.osm_data.id).await {
        Ok(new_osm_data) => match new_osm_data {
            Some(new_osm_data) => {
                if new_osm_data != user.osm_data {
                    info!(
                        old_osm_data = serde_json::to_string(&user.osm_data)?,
                        new_osm_data = serde_json::to_string(&new_osm_data)?,
                        "User data changed",
                    );
                    db::main::osm_user::queries::set_osm_data(user.id, new_osm_data, pool).await?;
                } else {
                    info!("User data didn't change")
                }

                let now = OffsetDateTime::now_utc();
                let now: String = now.format(&Rfc3339)?;
                db::main::osm_user::queries::set_tag(
                    user.id,
                    "osm:sync:date".into(),
                    Value::String(now),
                    pool,
                )
                .await?;
            }
            None => {
                warn!(user.osm_data.id, "User no longer exists on OSM");
                db::main::osm_user::queries::set_tag(
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
