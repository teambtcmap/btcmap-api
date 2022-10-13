use crate::db;
use crate::get_project_dirs;
use crate::model::Element;
use crate::model::User;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Statement;
use rusqlite::Transaction;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::ops::Sub;
use time::format_description::well_known::Rfc3339;
use time::Duration;
use time::OffsetDateTime;

pub async fn sync(mut db_conn: Connection) {
    log::info!("Starting sync");

    let project_dirs = get_project_dirs();
    let cache_dir = project_dirs.cache_dir();
    log::info!("Cache directory is {cache_dir:?}");

    if cache_dir.exists() {
        log::info!("Cache directory already exists");
    } else {
        log::info!("Cache directory doesn't exist, creating...");
        create_dir_all(cache_dir).expect("Failed to create cache directory");
        log::info!("Created cache directory {cache_dir:?}");
    }

    log::info!("Querying OSM API, it could take a while...");
    let response = reqwest::Client::new()
        .post("https://overpass-api.de/api/interpreter")
        .body(
            r#"
            [out:json][timeout:300];
            (
              node["currency:XBT"="yes"];
              way["currency:XBT"="yes"];
              relation["currency:XBT"="yes"];
              node["payment:bitcoin"="yes"];
              way["payment:bitcoin"="yes"];
              relation["payment:bitcoin"="yes"];
              node["currency:BTC"="yes"];
              way["currency:BTC"="yes"];
              relation["currency:BTC"="yes"];
            );
            out meta geom;
        "#,
        )
        .send()
        .await;

    if let Err(_) = response {
        log::error!("Failed to fetch response");
        std::process::exit(1);
    }

    let response = response.unwrap();
    log::info!("Fetched new data, response code: {}", response.status());

    let data_file_path = cache_dir.join("elements.json");
    log::info!("Data file path is {data_file_path:?}");

    let mut data_file = File::create(&data_file_path).expect("Failed to create data file");
    let response_body = response
        .bytes()
        .await
        .expect("Failed to read response body");
    data_file
        .write_all(&response_body)
        .expect("Failed to save new data to a file");

    let data_file = File::open(&data_file_path).expect("Failed to open data file");
    let fresh_elements: Value =
        serde_json::from_reader(data_file).expect("Failed to read data file into a JSON object");
    let fresh_elements: &Vec<Value> = fresh_elements["elements"]
        .as_array()
        .expect("Failed to extract elements");
    let nodes: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["type"].as_str().unwrap() == "node")
        .collect();

    let ways: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["type"].as_str().unwrap() == "way")
        .collect();

    let relations: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["type"].as_str().unwrap() == "relation")
        .collect();

    log::info!(
        "Got {} elements (nodes: {}, ways: {}, relations: {})",
        fresh_elements.len(),
        nodes.len(),
        ways.len(),
        relations.len()
    );

    if fresh_elements.len() < 5000 {
        log::error!("Data set is most likely invalid, skipping the sync");
        send_discord_message(
            "Got a suspicious resopnse from OSM, check server logs".to_string(),
        )
        .await;
        let suspicious_elements_file_path = cache_dir.join("suspicious-elements.json");
        std::fs::copy(&data_file_path, &suspicious_elements_file_path).unwrap();
        std::process::exit(1);
    }

    let onchain_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["tags"]["payment:onchain"].as_str().unwrap_or("") == "yes")
        .collect();

    let lightning_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["tags"]["payment:lightning"].as_str().unwrap_or("") == "yes")
        .collect();

    let lightning_contactless_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| {
            it["tags"]["payment:lightning_contactless"]
                .as_str()
                .unwrap_or("")
                == "yes"
        })
        .collect();

    let tx: Transaction = db_conn.transaction().unwrap();
    let mut elements_stmt: Statement = tx.prepare(db::ELEMENT_SELECT_ALL).unwrap();
    let elements: Vec<Element> = elements_stmt
        .query_map([], db::mapper_element_full())
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    drop(elements_stmt);
    log::info!("Found {} cached elements", elements.len());

    let fresh_element_ids: HashSet<String> = fresh_elements
        .iter()
        .map(|it| {
            format!(
                "{}:{}",
                it["type"].as_str().unwrap(),
                it["id"].as_i64().unwrap()
            )
        })
        .collect();

    let mut elements_created = 0;
    let mut elements_updated = 0;
    let mut elements_deleted = 0;

    // First, let's check if any of the cached elements no longer accept bitcoins
    for element in &elements {
        if !fresh_element_ids.contains(&element.id) && element.deleted_at.is_none() {
            let osm_id = element.data["id"].as_i64().unwrap();
            let element_type = element.data["type"].as_str().unwrap();
            let name = element.data["tags"]["name"]
                .as_str()
                .unwrap_or("Unnamed element");
            log::warn!("Cached element with id {} was deleted from OSM", element.id);

            let fresh_element = fetch_element(element_type, osm_id).await;
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
                db::ELEMENT_EVENT_INSERT,
                params![
                    OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                    element.id,
                    element.lat(),
                    element.lon(),
                    name,
                    "delete",
                    user_id,
                    user_display_name,
                ],
            )
            .unwrap();

            send_discord_message(format!(
                "{name} was deleted https://www.openstreetmap.org/{element_type}/{osm_id}"
            ))
            .await;
            let query =
                "UPDATE element SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = ?";
            log::info!("Executing query: {query:?}");
            tx.execute(query, params![element.id]).unwrap();
            elements_deleted += 1;
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
                let element_data: String = serde_json::to_string(&element.data).unwrap();
                let fresh_element_data = serde_json::to_string(fresh_element).unwrap();

                if element_data != fresh_element_data {
                    log::warn!("Element {btcmap_id} was updated");

                    insert_user_if_not_exists(user_id, &tx).await;

                    tx.execute(
                        db::ELEMENT_EVENT_INSERT,
                        params![
                            OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                            btcmap_id,
                            element.lat(),
                            element.lon(),
                            name,
                            "update",
                            user_id,
                            user_display_name,
                        ],
                    )
                    .unwrap();

                    send_discord_message(format!(
                        "{name} was updated by {user_display_name} https://www.openstreetmap.org/{element_type}/{osm_id}"
                    ))
                    .await;

                    tx.execute(
                        "UPDATE element SET data = ? WHERE id = ?",
                        params![fresh_element_data, btcmap_id],
                    )
                    .unwrap();

                    elements_updated += 1;
                }
            }
            None => {
                log::warn!("Element {btcmap_id} does not exist, inserting");

                insert_user_if_not_exists(user_id, &tx).await;

                let element = Element {
                    id: "".to_string(),
                    data: fresh_element.clone(),
                    created_at: "".to_string(),
                    updated_at: "".to_string(),
                    deleted_at: Option::None,
                };

                tx.execute(
                    db::ELEMENT_EVENT_INSERT,
                    params![
                        OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                        btcmap_id,
                        element.lat(),
                        element.lon(),
                        name,
                        "create",
                        user_id,
                        user_display_name,
                    ],
                )
                .unwrap();

                send_discord_message(format!(
                    "{name} was added by {user_display_name} https://www.openstreetmap.org/{element_type}/{osm_id}"
                ))
                .await;

                tx.execute(
                    "INSERT INTO element (id, data) VALUES (?, ?)",
                    params![btcmap_id, serde_json::to_string(fresh_element).unwrap()],
                )
                .unwrap();

                elements_created += 1;
            }
        }
    }

    let today = OffsetDateTime::now_utc().date();
    let year_ago = today.sub(Duration::days(365));
    log::info!("Today: {today}, year ago: {year_ago}");

    let up_to_date_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| {
            (it["tags"].get("survey:date").is_some()
                && it["tags"]["survey:date"].as_str().unwrap().to_string() > year_ago.to_string())
                || (it["tags"].get("check_date").is_some()
                    && it["tags"]["check_date"].as_str().unwrap().to_string()
                        > year_ago.to_string())
        })
        .collect();

    let outdated_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| {
            (it["tags"].get("check_date").is_none()
                && (it["tags"].get("survey:date").is_none()
                    || (it["tags"].get("survey:date").is_some()
                        && it["tags"]["survey:date"].as_str().unwrap().to_string()
                            <= year_ago.to_string())))
                || (it["tags"].get("survey:date").is_none()
                    && (it["tags"].get("check_date").is_none()
                        || (it["tags"].get("check_date").is_some()
                            && it["tags"]["check_date"].as_str().unwrap().to_string()
                                <= year_ago.to_string())))
        })
        .collect();

    let legacy_elements: Vec<&Value> = fresh_elements
        .iter()
        .filter(|it| it["tags"].get("payment:bitcoin").is_some())
        .collect();

    log::info!("Total elements: {}", elements.len());
    log::info!("Up to date elements: {}", up_to_date_elements.len());
    log::info!("Outdated elements: {}", outdated_elements.len());
    log::info!("Legacy elements: {}", legacy_elements.len());
    log::info!("Elements created: {elements_created}");
    log::info!("Elements updated: {elements_updated}");
    log::info!("Elements deleted: {elements_deleted}");

    let report = tx.query_row(
        db::DAILY_REPORT_SELECT_BY_AREA_ID_AND_DATE,
        params!["", today.to_string()],
        db::mapper_daily_report_full(),
    );

    if let Ok(report) = report {
        log::info!("Found existing report, updating");
        log::info!(
            "Existing report: created {}, updated {}, deleted {}",
            report.elements_created,
            report.elements_updated,
            report.elements_deleted
        );
        tx.execute(
            db::DAILY_REPORT_UPDATE_EVENT_COUNTERS,
            params![
                elements_created + report.elements_created,
                elements_updated + report.elements_updated,
                elements_deleted + report.elements_deleted,
                "",
                today.to_string(),
            ],
        )
        .unwrap();
    } else {
        log::info!("Inserting new report");
        tx.execute(
            db::DAILY_REPORT_INSERT,
            params![
                "",
                today.to_string(),
                fresh_elements.len(),
                onchain_elements.len(),
                lightning_elements.len(),
                lightning_contactless_elements.len(),
                up_to_date_elements.len(),
                outdated_elements.len(),
                legacy_elements.len(),
                elements_created,
                elements_updated,
                elements_deleted,
            ],
        )
        .unwrap();
    }

    tx.commit().expect("Failed to save sync results");
    log::info!("Finished sync");
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
    let url = format!("https://api.openstreetmap.org/api/0.6/{element_type}/{element_id}.json");
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
        .query_row(db::USER_SELECT_BY_ID, [user_id], db::mapper_user_full())
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
        db::USER_INSERT,
        params![user_id, serde_json::to_string(user.unwrap()).unwrap()],
    )
    .unwrap();
}
