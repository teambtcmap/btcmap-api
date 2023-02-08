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
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use tokio::time::sleep;
use tokio::time::Duration;

pub static OVERPASS_API_URL: &str = "https://z.overpass-api.de/api/interpreter";

pub static OVERPASS_API_QUERY: &str = r#"
    [out:json][timeout:300];
    (
    nwr["currency:XBT"="yes"];
    nwr["payment:bitcoin"="yes"];
    );
    out meta geom;
"#;

pub async fn run(mut db: Connection) -> Result<()> {
    log::info!("Starting sync");
    log::info!("Querying OSM API, it could take a while...");
    let response = reqwest::Client::new()
        .post(OVERPASS_API_URL)
        .body(OVERPASS_API_QUERY)
        .send()
        .await?;

    log::info!("Fetched new data, response code: {}", response.status());

    let response = response.json::<Value>().await?;

    let fresh_elements: &Vec<Value> = response["elements"]
        .as_array()
        .ok_or(Error::Other("Failed to parse elements".into()))?;

    log::info!("Fetched {} elements", fresh_elements.len());

    if fresh_elements.len() < 5000 {
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
        .query_map([], element::SELECT_ALL_MAPPER)
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    log::info!("Found {} cached elements", elements.len());

    let fresh_element_ids: HashSet<String> = fresh_elements
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

    for fresh_element in fresh_elements {
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
                let old_element_osm_json = serde_json::to_string(&element.osm_json)?;
                let new_element_osm_json = serde_json::to_string(fresh_element)?;

                if new_element_osm_json != old_element_osm_json {
                    log::info!("Element {btcmap_id} was updated");

                    insert_user_if_not_exists(user_id, &tx).await;

                    tx.execute(
                        event::INSERT,
                        named_params! {
                            ":user_id": user_id,
                            ":element_id": btcmap_id,
                            ":type": "update",
                        },
                    )?;

                    send_discord_message(format!(
                        "{name} was updated by {user_display_name} https://www.openstreetmap.org/{element_type}/{osm_id}"
                    ))
                    .await;

                    tx.execute(
                        element::UPDATE_OSM_JSON,
                        named_params! {
                            ":id": &btcmap_id,
                            ":osm_json": &new_element_osm_json,
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
                        ":osm_json": serde_json::to_string(fresh_element)?,
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
    log::info!("Finished sync");

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
