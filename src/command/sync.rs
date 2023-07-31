use crate::command::generate_android_icons::android_icon;
use crate::model::element;
use crate::model::event;
use crate::model::user;
use crate::model::Element;
use crate::model::User;
use crate::Error;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::time::SystemTime;
use tokio::time::sleep;
use tokio::time::Duration;
use tracing::error;
use tracing::info;
use tracing::warn;

pub static OVERPASS_API_URL: &str = "https://overpass-api.de/api/interpreter";

pub static OVERPASS_API_QUERY: &str = r#"
    [out:json][timeout:300];
    nwr["currency:XBT"=yes];
    out meta geom;
"#;

#[derive(Deserialize)]
struct OverpassJson {
    version: f64,
    generator: String,
    osm3s: Osm3s,
    elements: Vec<Value>,
}

#[derive(Deserialize)]
struct Osm3s {
    timestamp_osm_base: String,
}

pub async fn run(db: Connection) -> Result<()> {
    info!(db_path = ?db.path().unwrap(), "Starting sync");
    info!(
        OVERPASS_API_URL,
        OVERPASS_API_QUERY, "Querying Overpass API, it could take a while..."
    );

    let fetch_overpass_json_start = SystemTime::now();
    let json = fetch_overpass_json(OVERPASS_API_URL, OVERPASS_API_QUERY).await?;
    let fetch_overpass_json_duration = SystemTime::now()
        .duration_since(fetch_overpass_json_start)
        .unwrap();

    let process_overpass_json_start = SystemTime::now();
    process_overpass_json(json, db).await?;
    let process_overpass_json_duration = SystemTime::now()
        .duration_since(process_overpass_json_start)
        .unwrap();

    info!(
        fetch_overpass_json_duration_seconds = fetch_overpass_json_duration.as_secs_f64(),
        process_overpass_json_duration_seconds = process_overpass_json_duration.as_secs_f64(),
        "Finished sync",
    );

    Ok(())
}

async fn fetch_overpass_json(api_url: &str, query: &str) -> Result<OverpassJson> {
    let response = reqwest::Client::new()
        .post(api_url)
        .body(query.to_string())
        .send()
        .await?;

    info!(
        http_status_code = response.status().as_u16(),
        "Got response from Overpass"
    );

    let response = response.json::<OverpassJson>().await?;

    info!(
        response.version,
        response.generator,
        response.osm3s.timestamp_osm_base,
        elements = response.elements.len(),
        "Parsed Overpass response",
    );

    Ok(response)
}

async fn process_overpass_json(json: OverpassJson, mut db: Connection) -> Result<()> {
    if json.elements.len() < 5000 {
        let message = format!("Overpass returned {} elements", json.elements.len());
        error!(
            elements = json.elements.len(),
            discord_message = message,
            message,
        );
        Err(Error::Other(message.into()))?
    }

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

    let fresh_element_ids: HashSet<String> = json
        .elements
        .iter()
        .map(|it| {
            format!(
                "{}:{}",
                it["type"].as_str().unwrap(),
                it["id"].as_i64().unwrap(),
            )
        })
        .collect();

    // First, let's check if any of the cached elements no longer accept bitcoins
    for element in &elements {
        if !fresh_element_ids.contains(&element.id) && element.deleted_at.len() == 0 {
            let osm_id = element.osm_json["id"].as_i64().unwrap();
            let element_type = element.osm_json["type"].as_str().unwrap();
            let name = element.osm_json["tags"]["name"]
                .as_str()
                .unwrap_or("Unnamed element");
            warn!(
                element.id,
                "Cached element was deleted from Overpass or no longer accepts Bitcoin",
            );

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

    for fresh_element in json.elements {
        let element_type = fresh_element["type"].as_str().unwrap();
        let osm_id = fresh_element["id"].as_i64().unwrap();
        let btcmap_id = format!("{element_type}:{osm_id}");
        let name = fresh_element["tags"]["name"]
            .as_str()
            .unwrap_or("Unnamed element");
        let user_id = fresh_element["uid"].as_i64().unwrap_or(0);
        let user_display_name = fresh_element["user"].as_str().unwrap_or("");

        match elements.iter().find(|it| it.id == btcmap_id) {
            Some(element) => {
                if fresh_element != element.osm_json {
                    info!(
                        btcmap_id,
                        old_json = serde_json::to_string(&element.osm_json)?,
                        new_json = serde_json::to_string(&fresh_element)?,
                        "Element JSON was updated",
                    );

                    insert_user_if_not_exists(user_id, &tx).await;

                    if fresh_element["changeset"].as_i64() != element.osm_json["changeset"].as_i64()
                    {
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

                    let new_android_icon = android_icon(fresh_element["tags"].as_object().unwrap());
                    let old_android_icon = element
                        .tags
                        .get("icon:android")
                        .unwrap_or(&Value::Null)
                        .as_str()
                        .unwrap_or("");

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

                insert_user_if_not_exists(user_id, &tx).await;

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

                let category = element.category();
                let android_icon = android_icon(&element.osm_json["tags"].as_object().unwrap());

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

// #[cfg(test)]
// pub mod tests {
//     use super::{OVERPASS_API_QUERY, OVERPASS_API_URL};

//     #[actix_web::test]
//     async fn fetch_elements_from_osm() {
//         super::fetch_overpass_json(OVERPASS_API_URL, OVERPASS_API_QUERY)
//             .await
//             .unwrap();
//     }
// }
