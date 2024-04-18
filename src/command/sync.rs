use crate::area::Area;
use crate::discord;
use crate::element::find_areas;
use crate::element::Element;
use crate::event::Event;
use crate::lint;
use crate::osm::osm;
use crate::osm::overpass::query_bitcoin_merchants;
use crate::osm::overpass::OverpassElement;
use crate::user::User;
use crate::Error;
use crate::Result;
use rusqlite::Connection;
use rusqlite::Transaction;
use serde_json::Value;
use std::collections::HashSet;
use std::ops::Add;
use std::time::SystemTime;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn run(db: Connection) -> Result<()> {
    info!(db_path = ?db.path().unwrap(), "Starting sync");

    let query_elements_start = SystemTime::now();
    let elements = query_bitcoin_merchants().await?;
    let query_elements_duration = SystemTime::now()
        .duration_since(query_elements_start)
        .unwrap();

    let process_elements_start = SystemTime::now();
    process_elements(elements, db).await?;
    let process_elements_duration = SystemTime::now()
        .duration_since(process_elements_start)
        .unwrap();

    info!(
        query_elements_duration_seconds = query_elements_duration.as_secs_f64(),
        process_elements_duration_seconds = process_elements_duration.as_secs_f64(),
        "Finished sync",
    );

    Ok(())
}

async fn process_elements(fresh_elements: Vec<OverpassElement>, mut db: Connection) -> Result<()> {
    let tx: Transaction = db.transaction()?;

    let cached_elements = Element::select_all(None, &tx)?;

    info!(db_path = ?tx.path().unwrap(), elements = cached_elements.len(), "Loaded all elements from database");

    let fresh_element_ids: HashSet<String> = fresh_elements
        .iter()
        .map(|it| format!("{}:{}", it.r#type, it.id,))
        .collect();

    info!("Loading areas");
    let areas: Vec<Area> = Area::select_all(None, &tx)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();

    // First, let's check if any of the cached elements no longer accept bitcoins
    for cached_element in &cached_elements {
        if !fresh_element_ids.contains(&cached_element.overpass_data.btcmap_id())
            && cached_element.deleted_at.is_none()
        {
            warn!(
                cached_element.id,
                "Cached element was deleted from Overpass or no longer accepts Bitcoin",
            );
            let osm_id = cached_element.overpass_data.id;
            let element_type = &cached_element.overpass_data.r#type;
            let name = cached_element.overpass_data.tag("name");

            let fresh_element = match osm::get_element(element_type, osm_id).await? {
                Some(fresh_element) => fresh_element,
                None => Err(Error::OsmApi(format!(
                    "Failed to fetch element {element_type}:{osm_id} from OSM"
                )))?,
            };

            if fresh_element.visible.unwrap_or(true) {
                if fresh_element.tag("currency:XBT", "no") == "yes" {
                    let message = format!(
                        "Overpass lied about element {element_type}:{osm_id} being deleted"
                    );
                    error!(element_type, osm_id, message);
                    discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;
                    Err(Error::OverpassApi(message.into()))?
                }
            }

            insert_user_if_not_exists(fresh_element.uid, &tx).await?;

            let event = Event::insert(fresh_element.uid, cached_element.id, "delete", &tx)?;
            on_new_event(&event, &tx).await?;

            let message = format!(
                "User {} removed https://www.openstreetmap.org/{element_type}/{osm_id}",
                fresh_element.user
            );
            info!(
                element_name = name,
                element_url = format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                user_name = fresh_element.user,
                message,
            );
            discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;

            info!(cached_element.id, "Marking element as deleted");
            cached_element.set_deleted_at(Some(OffsetDateTime::now_utc()), &tx)?;
        }
    }

    for fresh_element in fresh_elements {
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
                        insert_user_if_not_exists(user_id, &tx).await?;
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
                        updated_element = updated_element.set_tag(
                            "icon:android",
                            &new_android_icon.clone().into(),
                            &tx,
                        )?;
                    }

                    lint::generate_element_issues(&updated_element, &tx)?;
                    find_areas::find_and_save(&updated_element, &areas, &tx)?;
                }

                if cached_element.deleted_at.is_some() {
                    info!(btcmap_id, "Bitcoin tags were re-added");
                    cached_element.set_deleted_at(None, &tx)?;
                }
            }
            None => {
                info!(btcmap_id, "Element does not exist, inserting");

                if let Some(user_id) = user_id {
                    insert_user_if_not_exists(user_id, &tx).await?;
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

                let element = element.set_tag("category", &category.clone().into(), &tx)?;
                let element = element.set_tag("icon:android", &android_icon.clone().into(), &tx)?;

                info!(category, android_icon);

                lint::generate_element_issues(&element, &tx)?;
                find_areas::find_and_save(&element, &areas, &tx)?;

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
    Ok(())
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

    Ok(())
}

pub async fn insert_user_if_not_exists(user_id: i64, conn: &Connection) -> Result<()> {
    let db_user = User::select_by_id(user_id, conn)?;

    if db_user.is_some() {
        info!(user_id, "User already exists");
        return Ok(());
    }

    let user = osm::get_user(user_id).await?;

    match user {
        Some(user) => User::insert(user_id, &user, &conn)?,
        None => Err(Error::OsmApi(format!(
            "User with id = {user_id} doesn't exist on OSM"
        )))?,
    };

    Ok(())
}
