use crate::model::event::Event;
use crate::model::user;
use crate::model::Element;
use crate::model::OverpassElementJson;
use crate::model::User;
use crate::service::overpass::query_bitcoin_merchants;
use crate::Error;
use crate::Result;
use reqwest::StatusCode;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
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

async fn process_elements(
    fresh_elements: Vec<OverpassElementJson>,
    mut db: Connection,
) -> Result<()> {
    let tx: Transaction = db.transaction()?;

    let elements = Element::select_all(None, &tx)?;

    info!(db_path = ?tx.path().unwrap(), elements = elements.len(), "Loaded all elements from database");

    let fresh_element_ids: HashSet<String> = fresh_elements
        .iter()
        .map(|it| format!("{}:{}", it.r#type, it.id,))
        .collect();

    // First, let's check if any of the cached elements no longer accept bitcoins
    for element in &elements {
        if !fresh_element_ids.contains(&element.id) && element.deleted_at.is_none() {
            warn!(
                element.id,
                "Cached element was deleted from Overpass or no longer accepts Bitcoin",
            );
            let osm_id = element.overpass_json.id;
            let element_type = &element.overpass_json.r#type;
            let name = element.get_osm_tag_value("name");

            let fresh_element = match fetch_element(element_type, osm_id).await? {
                Some(fresh_element) => fresh_element,
                None => Err(Error::Other(format!(
                    "Failed to fetch element {element_type}:{osm_id} from OSM"
                )))?,
            };

            let default_bitcoin_tag_value = "no".to_string();

            if fresh_element.visible.unwrap_or(true) {
                let bitcoin_tag_value = fresh_element
                    .tags
                    .get("currency:XBT")
                    .unwrap_or(&default_bitcoin_tag_value);
                info!(bitcoin_tag_value);

                if bitcoin_tag_value == "yes" {
                    let message = format!(
                        "Overpass lied about element {element_type}:{osm_id} being deleted"
                    );
                    error!(element_type, osm_id, discord_message = message, message,);
                    Err(Error::Other(message.into()))?
                }
            }

            insert_user_if_not_exists(fresh_element.uid, &tx).await;

            Event::insert(fresh_element.uid, &element.id, "delete", &tx)?;

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

            info!(element.id, "Marking element as deleted");
            Element::set_deleted_at(&element.id, Some(OffsetDateTime::now_utc()), &tx)?;
        }
    }

    for fresh_element in fresh_elements {
        let element_type = &fresh_element.r#type;
        let osm_id = fresh_element.id;
        let btcmap_id = fresh_element.btcmap_id();
        let name = fresh_element.get_tag_value("name");
        let user_id = fresh_element.uid;
        let user_display_name = &fresh_element.user.clone().unwrap_or_default();

        match elements.iter().find(|it| it.id == btcmap_id) {
            Some(element) => {
                if fresh_element != element.overpass_json {
                    info!(
                        btcmap_id,
                        old_json = serde_json::to_string(&element.overpass_json)?,
                        new_json = serde_json::to_string(&fresh_element)?,
                        "Element JSON was updated",
                    );

                    if let Some(user_id) = user_id {
                        insert_user_if_not_exists(user_id, &tx).await;
                    }

                    if fresh_element.changeset != element.overpass_json.changeset {
                        Event::insert(
                            user_id.unwrap().try_into().unwrap(),
                            &btcmap_id,
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
                    Element::set_overpass_json(&btcmap_id, &fresh_element, &tx)?;

                    let new_android_icon = fresh_element.generate_android_icon();
                    let old_android_icon = element.get_btcmap_tag_value_str("icon:android");

                    if new_android_icon != old_android_icon {
                        info!(old_android_icon, new_android_icon, "Updating Android icon");
                        Element::insert_tag(&element.id, "icon:android", &new_android_icon, &tx)?;
                    }
                }

                if element.deleted_at.is_some() {
                    info!(btcmap_id, "Bitcoin tags were re-added");
                    Element::set_deleted_at(&btcmap_id, None, &tx)?;
                }
            }
            None => {
                info!(btcmap_id, "Element does not exist, inserting");

                if let Some(user_id) = user_id {
                    insert_user_if_not_exists(user_id, &tx).await;
                }

                Element::insert(&fresh_element, &tx)?;

                Event::insert(
                    user_id.unwrap().try_into().unwrap(),
                    &btcmap_id,
                    "create",
                    &tx,
                )?;

                let element = Element::select_by_id(&btcmap_id, &tx)?.unwrap();
                let category = element.generate_category();
                let android_icon = element.generate_android_icon();

                Element::insert_tag(&element.id, "category", &category, &tx)?;
                Element::insert_tag(&element.id, "icon:android", &android_icon, &tx)?;

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

#[derive(Deserialize)]
struct OsmResponseJson {
    elements: Vec<OsmElementJson>,
}

#[derive(Deserialize)]
struct OsmElementJson {
    //r#type: String,
    //id: i64,
    visible: Option<bool>,
    tags: HashMap<String, String>,
    user: String,
    uid: i32,
}

async fn fetch_element(element_type: &str, element_id: i64) -> Result<Option<OsmElementJson>> {
    let url = format!(
        "https://api.openstreetmap.org/api/0.6/{element_type}s.json?{element_type}s={element_id}"
    );
    info!(url, "Querying OSM");

    let res = reqwest::get(&url).await?;

    if res.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let mut res: OsmResponseJson = res.json().await?;

    if res.elements.len() == 1 {
        return Ok(Some(res.elements.pop().unwrap()));
    } else {
        return Ok(None);
    }
}

pub async fn insert_user_if_not_exists(user_id: i32, conn: &Connection) {
    if user_id == 0 {
        return;
    }

    let db_user: Option<User> = conn
        .query_row(
            user::SELECT_BY_ID,
            &[(":id", &user_id)],
            user::SELECT_BY_ID_MAPPER,
        )
        .optional()
        .unwrap();

    if db_user.is_some() {
        info!(user_id, "User already exists");
        return;
    }

    let url = format!("https://api.openstreetmap.org/api/0.6/user/{user_id}.json");
    info!(url, "Querying OSM");
    let res = reqwest::get(&url).await;

    if let Err(_) = res {
        error!(user_id, "Failed to fetch user");
        return;
    }

    let body = res.unwrap().text().await;

    if let Err(_) = body {
        error!(user_id, "Failed to fetch user");
        return;
    }

    let body: serde_json::Result<Value> = serde_json::from_str(&body.unwrap());

    if let Err(_) = body {
        error!(user_id, "Failed to fetch user");
        return;
    }

    let body = body.unwrap();
    let user: Option<&Value> = body.get("user");

    if user.is_none() {
        error!(user_id, "Failed to fetch user");
        return;
    }

    conn.execute(
        user::INSERT,
        named_params! {
            ":id": user_id,
            ":osm_json": serde_json::to_string(user.unwrap()).unwrap()
        },
    )
    .unwrap();
}
