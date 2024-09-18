use crate::discord;
use crate::element;
use crate::element::Element;
use crate::event::Event;
use crate::osm::osm;
use crate::osm::overpass::query_bitcoin_merchants;
use crate::osm::overpass::OverpassElement;
use crate::sync;
use crate::user::User;
use crate::Result;
use rusqlite::Connection;
use rusqlite::Transaction;
use serde_json::Value;
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn run(conn: &mut Connection) -> Result<()> {
    let elements = query_bitcoin_merchants().await?;
    let res = merge_overpass_elements(elements, conn).await?;
    info!(res.elements_deleted);
    Ok(())
}

pub struct MergeResult {
    pub elements_deleted: usize,
}

async fn merge_overpass_elements(
    fresh_overpass_elements: Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<MergeResult> {
    let deleted_element_events = sync::sync_deleted_elements(
        &fresh_overpass_elements
            .iter()
            .map(|it| it.btcmap_id())
            .collect(),
        conn,
    )
    .await?;
    for event in &deleted_element_events {
        on_new_event(&event, conn).await?;
    }
    let tx: Transaction = conn.transaction()?;
    let cached_elements = Element::select_all(None, &tx)?;
    for fresh_element in fresh_overpass_elements {
        let element_type = &fresh_element.r#type;
        let osm_id = fresh_element.id;
        let btcmap_id = fresh_element.btcmap_id();
        let name = fresh_element.tag("name");
        let user_id = fresh_element.uid;
        let user_display_name = &fresh_element.user.clone().unwrap_or_default();

        match cached_elements
            .iter()
            .find(|it| it.overpass_data.btcmap_id() == btcmap_id)
        {
            Some(cached_element) => {
                if fresh_element != cached_element.overpass_data {
                    info!(
                        btcmap_id,
                        old_json = serde_json::to_string(&cached_element.overpass_data)?,
                        new_json = serde_json::to_string(&fresh_element)?,
                        "Element JSON was updated",
                    );

                    if let Some(user_id) = user_id {
                        sync::insert_user_if_not_exists(user_id, &tx).await?;
                    }

                    if fresh_element.changeset != cached_element.overpass_data.changeset {
                        let event = Event::insert(
                            user_id.unwrap().try_into().unwrap(),
                            cached_element.id,
                            "update",
                            &tx,
                        )?;
                        on_new_event(&event, &tx).await?;
                    } else {
                        warn!("Changeset ID is identical, skipped user event generation");
                    }

                    let message = format!("User {user_display_name} updated https://www.openstreetmap.org/{element_type}/{osm_id}");
                    info!(
                        element_name = name,
                        element_url =
                            format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                        user_name = user_display_name,
                        message,
                    );
                    discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;

                    info!("Updating osm_json");
                    let mut updated_element =
                        cached_element.set_overpass_data(&fresh_element, &tx)?;

                    let new_android_icon = updated_element.overpass_data.generate_android_icon();
                    let old_android_icon = cached_element
                        .tag("icon:android")
                        .as_str()
                        .unwrap_or_default();

                    if new_android_icon != old_android_icon {
                        info!(old_android_icon, new_android_icon, "Updating Android icon");
                        updated_element = Element::set_tag(
                            updated_element.id,
                            "icon:android",
                            &new_android_icon.clone().into(),
                            &tx,
                        )?;
                    }

                    element::service::generate_issues(vec![&updated_element], &tx)?;
                    element::service::generate_areas_mapping_old(&vec![updated_element], &tx)?;
                }

                if cached_element.deleted_at.is_some() {
                    info!(btcmap_id, "Bitcoin tags were re-added");
                    cached_element.set_deleted_at(None, &tx)?;
                }
            }
            None => {
                info!(btcmap_id, "Element does not exist, inserting");

                if let Some(user_id) = user_id {
                    sync::insert_user_if_not_exists(user_id, &tx).await?;
                }

                let element = Element::insert(&fresh_element, &tx)?;

                let event = Event::insert(
                    user_id.unwrap().try_into().unwrap(),
                    element.id,
                    "create",
                    &tx,
                )?;
                on_new_event(&event, &tx).await?;

                let category = element.overpass_data.generate_category();
                let android_icon = element.overpass_data.generate_android_icon();

                let element =
                    Element::set_tag(element.id, "category", &category.clone().into(), &tx)?;
                let element = Element::set_tag(
                    element.id,
                    "icon:android",
                    &android_icon.clone().into(),
                    &tx,
                )?;

                info!(category, android_icon);

                element::service::generate_issues(vec![&element], &tx)?;
                element::service::generate_areas_mapping_old(&vec![element], &tx)?;

                let message = format!("User {user_display_name} added https://www.openstreetmap.org/{element_type}/{osm_id}");
                info!(
                    element_name = name,
                    element_category = category,
                    element_android_icon = android_icon,
                    element_url = format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                    user_name = user_display_name,
                    message,
                );
                discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;
            }
        }
    }

    tx.commit()?;
    Ok(MergeResult {
        elements_deleted: deleted_element_events.len(),
    })
}

async fn on_new_event(event: &Event, conn: &Connection) -> Result<()> {
    let user = User::select_by_id(event.user_id, &conn)?.unwrap();

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

    match osm::get_user(user.osm_data.id).await {
        Ok(new_osm_data) => match new_osm_data {
            Some(new_osm_data) => {
                if new_osm_data != user.osm_data {
                    info!(
                        old_osm_data = serde_json::to_string(&user.osm_data)?,
                        new_osm_data = serde_json::to_string(&new_osm_data)?,
                        "User data changed",
                    );
                    User::set_osm_data(user.id, &new_osm_data, &conn)?;
                } else {
                    info!("User data didn't change")
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

    if event.r#type == "delete" {
        let message = format!(
            "User {} removed https://www.openstreetmap.org/{}/{}",
            user.osm_data.display_name, event.element_osm_type, event.element_osm_id
        );
        info!(message);
        discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;
    }

    Ok(())
}
