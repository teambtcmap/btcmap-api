use crate::model::element;
use crate::model::event;
use crate::model::user;
use crate::model::Element;
use crate::model::OverpassElement;
use crate::model::User;
use crate::service::overpass::query_bitcoin_merchants;
use crate::Error;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use serde_json::Value;
use std::collections::HashSet;
use std::time::SystemTime;
use tokio::time::sleep;
use tokio::time::Duration;
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
    let elements: Vec<Element> = tx
        .prepare(element::SELECT_ALL)
        .unwrap()
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            element::SELECT_ALL_MAPPER,
        )
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    info!(db_path = ?tx.path().unwrap(), elements = elements.len(), "Loaded all elements from database");

    let fresh_element_ids: HashSet<String> = fresh_elements
        .iter()
        .map(|it| format!("{}:{}", it.r#type, it.id,))
        .collect();

    // First, let's check if any of the cached elements no longer accept bitcoins
    for element in &elements {
        if !fresh_element_ids.contains(&element.id) && element.deleted_at.len() == 0 {
            warn!(
                element.id,
                "Cached element was deleted from Overpass or no longer accepts Bitcoin",
            );
            let osm_id = element.osm_json.id;
            let element_type = &element.osm_json.r#type;
            let name = element.get_osm_tag_value("name");

            let fresh_element = fetch_element(element_type, osm_id).await;
            let deleted_from_osm = !fresh_element
                .clone()
                .map(|it| it["visible"].as_bool().unwrap_or(true))
                .unwrap_or(true);
            info!(deleted_from_osm);

            if !deleted_from_osm {
                let fresh_element = fresh_element.clone().unwrap();
                let bitcoin_tag_value = fresh_element["tags"]["currency:XBT"]
                    .as_str()
                    .unwrap_or("no");
                info!(bitcoin_tag_value);

                if bitcoin_tag_value == "yes" {
                    let message = format!(
                        "Overpass lied about element {element_type}:{osm_id} being deleted"
                    );
                    error!(element_type, osm_id, discord_message = message, message,);
                    Err(Error::Other(message.into()))?
                }
            }

            let user_id = fresh_element
                .clone()
                .map(|it| it["uid"].as_i64().unwrap_or(0))
                .unwrap_or(0);
            let user_display_name = fresh_element
                .clone()
                .map(|it| it["user"].as_str().unwrap_or("").to_string())
                .unwrap_or("".to_string());

            insert_user_if_not_exists(user_id, &tx).await;

            tx.execute(
                event::INSERT,
                named_params! {
                    ":user_id": user_id,
                    ":element_id": element.id,
                    ":type": "delete",
                },
            )?;

            let message = format!("User {user_display_name} removed https://www.openstreetmap.org/{element_type}/{osm_id}");
            info!(
                element_name = name,
                element_url = format!("https://www.openstreetmap.org/{element_type}/{osm_id}"),
                user_name = user_display_name,
                discord_message = message,
                message,
            );

            info!(element.id, "Marking element as deleted");
            tx.execute(
                element::MARK_AS_DELETED,
                named_params! { ":id": element.id },
            )?;
        }
    }

    for fresh_element in fresh_elements {
        let element_type = &fresh_element.r#type;
        let osm_id = fresh_element.id;
        let btcmap_id = fresh_element.btcmap_id();
        let name = fresh_element.get_tag_value("name");
        let user_id = fresh_element.uid;
        let user_display_name = &fresh_element.user.clone().unwrap_or("".into());

        match elements.iter().find(|it| it.id == btcmap_id) {
            Some(element) => {
                if fresh_element != element.osm_json {
                    info!(
                        btcmap_id,
                        old_json = serde_json::to_string(&element.osm_json)?,
                        new_json = serde_json::to_string(&fresh_element)?,
                        "Element JSON was updated",
                    );

                    if let Some(user_id) = user_id {
                        insert_user_if_not_exists(user_id, &tx).await;
                    }

                    if fresh_element.changeset != element.osm_json.changeset {
                        tx.execute(
                            event::INSERT,
                            named_params! {
                                ":user_id": user_id,
                                ":element_id": btcmap_id,
                                ":type": "update",
                            },
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

                    tx.execute(
                        element::UPDATE_OSM_JSON,
                        named_params! {
                            ":id": &btcmap_id,
                            ":osm_json": serde_json::to_string(&fresh_element)?,
                        },
                    )?;

                    let new_android_icon = fresh_element.generate_android_icon();
                    let old_android_icon = element.get_btcmap_tag_value_str("icon:android");

                    if new_android_icon != old_android_icon {
                        info!(old_android_icon, new_android_icon, "Updating Android icon");

                        tx.execute(
                            element::INSERT_TAG,
                            named_params! {
                                ":element_id": &element.id,
                                ":tag_name": "$.icon:android",
                                ":tag_value": &new_android_icon,
                            },
                        )?;
                    }
                }

                if element.deleted_at.len() > 0 {
                    tx.execute(
                        element::UPDATE_DELETED_AT,
                        named_params! {
                            ":id": &btcmap_id,
                            ":deleted_at": "",
                        },
                    )?;
                }
            }
            None => {
                info!(btcmap_id, "Element does not exist, inserting");

                if let Some(user_id) = user_id {
                    insert_user_if_not_exists(user_id, &tx).await;
                }

                tx.execute(
                    event::INSERT,
                    named_params! {
                        ":user_id": user_id,
                        ":element_id": btcmap_id,
                        ":type": "create",
                    },
                )?;

                tx.execute(
                    element::INSERT,
                    named_params! {
                        ":id": &btcmap_id,
                        ":osm_json": serde_json::to_string(&fresh_element)?,
                    },
                )?;

                let element = tx.query_row(
                    element::SELECT_BY_ID,
                    &[(":id", &btcmap_id)],
                    element::SELECT_BY_ID_MAPPER,
                )?;

                let category = element.generate_category();
                let android_icon = element.generate_android_icon();

                tx.execute(
                    element::INSERT_TAG,
                    named_params! {
                        ":element_id": &element.id,
                        ":tag_name": "$.category",
                        ":tag_value": &category,
                    },
                )?;

                tx.execute(
                    element::INSERT_TAG,
                    named_params! {
                        ":element_id": &element.id,
                        ":tag_name": "$.icon:android",
                        ":tag_value": &android_icon,
                    },
                )?;

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

async fn fetch_element(element_type: &str, element_id: i64) -> Option<Value> {
    sleep(Duration::from_millis(1000)).await;

    let url = format!(
        "https://api.openstreetmap.org/api/0.6/{element_type}s.json?{element_type}s={element_id}"
    );
    info!(url, "Querying OSM");
    let res = reqwest::get(&url).await;

    if let Err(_) = res {
        error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body = res.unwrap().text().await;

    if let Err(_) = body {
        error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body: serde_json::Result<Value> = serde_json::from_str(&body.unwrap());

    if let Err(_) = body {
        error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body = body.unwrap();
    let elements: Option<&Vec<Value>> = body["elements"].as_array();

    if elements.is_none() || elements.unwrap().len() == 0 {
        error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    Some(elements.unwrap()[0].clone())
}

pub async fn insert_user_if_not_exists(user_id: i64, conn: &Connection) {
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
