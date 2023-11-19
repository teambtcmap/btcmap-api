use crate::element::Element;
use crate::event::Event;
use crate::osm::osm;
use crate::osm::overpass::query_bitcoin_merchants;
use crate::osm::overpass::OverpassElement;
use crate::user::User;
use crate::Error;
use crate::Result;
use rusqlite::Connection;
use rusqlite::Transaction;
use std::collections::HashSet;
use std::time::SystemTime;
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
                    error!(element_type, osm_id, discord_message = message, message,);
                    Err(Error::OverpassApi(message.into()))?
                }
            }

            insert_user_if_not_exists(fresh_element.uid, &tx).await?;

            Event::insert(fresh_element.uid, cached_element.id, "delete", &tx)?;

            let message = format!(
                "User {} removed https://www.openstreetmap.org/{element_type}/{osm_id}",
                fresh_element.user
            );
            info!(
                element_name = name,
                element_url = format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                user_name = fresh_element.user,
                discord_message = message,
                message,
            );

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
                        Event::insert(
                            user_id.unwrap().try_into().unwrap(),
                            cached_element.id,
                            "update",
                            &tx,
                        )?;
                    } else {
                        warn!("Changeset ID is identical, skipped user event generation");
                    }

                    let message = format!("User {user_display_name} updated https://www.openstreetmap.org/{element_type}/{osm_id}");
                    info!(
                        element_name = name,
                        element_url =
                            format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                        user_name = user_display_name,
                        discord_message = message,
                        message,
                    );

                    info!("Updating osm_json");
                    cached_element.set_overpass_data(&fresh_element, &tx)?;

                    let new_android_icon = fresh_element.generate_android_icon();
                    let old_android_icon = cached_element
                        .tag("icon:android")
                        .as_str()
                        .unwrap_or_default();

                    if new_android_icon != old_android_icon {
                        info!(old_android_icon, new_android_icon, "Updating Android icon");
                        cached_element.set_tag(
                            "icon:android",
                            &new_android_icon.clone().into(),
                            &tx,
                        )?;
                    }
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

                Event::insert(
                    user_id.unwrap().try_into().unwrap(),
                    element.id,
                    "create",
                    &tx,
                )?;

                let category = element.overpass_data.generate_category();
                let android_icon = element.overpass_data.generate_android_icon();

                element.set_tag("category", &category.clone().into(), &tx)?;
                element.set_tag("icon:android", &android_icon.clone().into(), &tx)?;

                info!(category, android_icon);

                let message = format!("User {user_display_name} added https://www.openstreetmap.org/{element_type}/{osm_id}");
                info!(
                    element_name = name,
                    element_category = category,
                    element_android_icon = android_icon,
                    element_url = format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                    user_name = user_display_name,
                    discord_message = message,
                    message,
                );
            }
        }
    }

    tx.commit()?;
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
