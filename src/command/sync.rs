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
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use tokio::time::sleep;
use tokio::time::Duration;

pub static OVERPASS_API_URL: &str = "https://overpass-api.de/api/interpreter";

pub static OVERPASS_API_QUERY: &str = r#"
    [out:json][timeout:300];
    (
    nwr["currency:XBT"="yes"];
    nwr["payment:bitcoin"="yes"];
    );
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
    log::info!(
        "{}",
        json!({
            "message": "Starting sync",
            "db_location": db.path().unwrap(),
        }),
    );

    log::info!(
        "{}",
        json!({
            "message": "Querying Overpass API, it could take a while...",
            "api_url": OVERPASS_API_URL,
            "query": OVERPASS_API_QUERY,
        }),
    );

    let json = fetch_overpass_json(OVERPASS_API_URL, OVERPASS_API_QUERY).await?;
    process_overpass_json(json, db).await?;

    log::info!("{}", json!({ "message": "Finished sync" }));
    Ok(())
}

async fn fetch_overpass_json(api_url: &str, query: &str) -> Result<OverpassJson> {
    let response = reqwest::Client::new()
        .post(api_url)
        .body(query.to_string())
        .send()
        .await?;

    log::info!(
        "{}",
        json!(
            {
                "message": "Got response from Overpass",
                "http_status_code": response.status().as_u16(),
            }
        )
    );

    let response = response.json::<OverpassJson>().await?;

    log::info!(
        "{}",
        json!(
            {
                "message": "Parsed Overpass response",
                "response_version": response.version,
                "generator": response.generator,
                "timestamp_osm_base": response.osm3s.timestamp_osm_base,
                "elements": response.elements.len(),
            }
        )
    );

    Ok(response)
}

async fn process_overpass_json(json: OverpassJson, mut db: Connection) -> Result<()> {
    if json.elements.len() < 5000 {
        send_discord_message("Got a suspicious resopnse from OSM, check server logs".to_string())
            .await;
        Err(Error::Other(
            "Data set is most likely invalid, skipping the sync".into(),
        ))?
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

    log::info!("Found {} cached elements", elements.len());

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
            log::warn!(
                "Cached element with id {} was deleted from Overpass or no longer accepts Bitcoin",
                element.id
            );

            let fresh_element = fetch_element(element_type, osm_id).await;
            let deleted_from_osm = !fresh_element
                .clone()
                .map(|it| it["visible"].as_bool().unwrap_or(true))
                .unwrap_or(true);
            log::info!("Deleted from OSM: {deleted_from_osm}");

            if !deleted_from_osm {
                let fresh_element = fresh_element.clone().unwrap();
                let bitcoin_tag_value = fresh_element["tags"]["currency:XBT"]
                    .as_str()
                    .unwrap_or("no");
                let legacy_bitcoin_tag_value = fresh_element["tags"]["payment:bitcoin"]
                    .as_str()
                    .unwrap_or("no");
                log::info!("Bitcoin tag value: {bitcoin_tag_value}");
                log::info!("Legacy Bitcoin tag value: {legacy_bitcoin_tag_value}");

                if bitcoin_tag_value == "yes" || legacy_bitcoin_tag_value == "yes" {
                    let message =
                        format!("Overpass lied about {element_type}/{osm_id} being deleted!");
                    send_discord_message(message.clone()).await;
                    Err(Error::Other(message))?
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

            send_discord_message(format!(
                "{name} was deleted by {user_display_name} https://www.openstreetmap.org/{element_type}/{osm_id}"
            ))
            .await;
            log::info!("Marking element {} as deleted", element.id);
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
                    log::info!("JSON for element {btcmap_id} was updated");
                    log::info!("Old JSON: {}", serde_json::to_string(&element.osm_json)?);
                    log::info!("New JSON: {}", serde_json::to_string(&fresh_element)?);

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
                        log::warn!("Changeset ID is identical, skipped user event generation");
                    }

                    send_discord_message(format!(
                        "{name} was updated by {user_display_name} https://www.openstreetmap.org/{element_type}/{osm_id}"
                    ))
                    .await;

                    tx.execute(
                        element::UPDATE_OSM_JSON,
                        named_params! {
                            ":id": &btcmap_id,
                            ":osm_json": serde_json::to_string(&fresh_element)?,
                        },
                    )?;
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
                log::info!("Element {btcmap_id} does not exist, inserting");

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

                let category_singular = element.category_singular();
                let category_plural = element.category_plural();
                let android_icon = element.android_icon();

                tx.execute(
                    element::INSERT_TAG,
                    named_params! {
                        ":element_id": &element.id,
                        ":tag_name": "$.category",
                        ":tag_value": &category_singular,
                    },
                )?;

                tx.execute(
                    element::INSERT_TAG,
                    named_params! {
                        ":element_id": &element.id,
                        ":tag_name": "$.category:plural",
                        ":tag_value": &category_plural,
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

                log::info!("Category: {category_singular}, icon: {android_icon}");

                send_discord_message(format!(
                    "{name} was added by {user_display_name} (category: {category_singular}, icon: {android_icon}) https://www.openstreetmap.org/{element_type}/{osm_id}"
                ))
                .await;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

async fn send_discord_message(text: String) {
    if let Ok(discord_webhook_url) = env::var("DISCORD_WEBHOOK_URL") {
        log::info!("Sending Discord message");
        let mut args = HashMap::new();
        args.insert("username", "btcmap.org".to_string());
        args.insert("content", text);

        let response = reqwest::Client::new()
            .post(discord_webhook_url)
            .json(&args)
            .send()
            .await;

        match response {
            Ok(response) => {
                log::info!("Discord response status: {:?}", response.status());
            }
            Err(_) => {
                log::error!("Failed to send Discord message");
            }
        }
    }
}

async fn fetch_element(element_type: &str, element_id: i64) -> Option<Value> {
    sleep(Duration::from_millis(1000)).await;

    let url = format!(
        "https://api.openstreetmap.org/api/0.6/{element_type}s.json?{element_type}s={element_id}"
    );
    log::info!("Querying {url}");
    let res = reqwest::get(&url).await;

    if let Err(_) = res {
        log::error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body = res.unwrap().text().await;

    if let Err(_) = body {
        log::error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body: serde_json::Result<Value> = serde_json::from_str(&body.unwrap());

    if let Err(_) = body {
        log::error!("Failed to fetch element {element_type}:{element_id}");
        return None;
    }

    let body = body.unwrap();
    let elements: Option<&Vec<Value>> = body["elements"].as_array();

    if elements.is_none() || elements.unwrap().len() == 0 {
        log::error!("Failed to fetch element {element_type}:{element_id}");
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
        log::info!("User {user_id} already exists");
        return;
    }

    let url = format!("https://api.openstreetmap.org/api/0.6/user/{user_id}.json");
    log::info!("Querying {url}");
    let res = reqwest::get(&url).await;

    if let Err(_) = res {
        log::error!("Failed to fetch user {user_id}");
        return;
    }

    let body = res.unwrap().text().await;

    if let Err(_) = body {
        log::error!("Failed to fetch user {user_id}");
        return;
    }

    let body: serde_json::Result<Value> = serde_json::from_str(&body.unwrap());

    if let Err(_) = body {
        log::error!("Failed to fetch user {user_id}");
        return;
    }

    let body = body.unwrap();
    let user: Option<&Value> = body.get("user");

    if user.is_none() {
        log::error!("Failed to fetch user {user_id}");
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
