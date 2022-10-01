use crate::db;
use crate::get_project_dirs;
use crate::model::Element;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::Statement;
use rusqlite::Transaction;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::ops::Sub;
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

    log::info!("Fetching new data...");

    let response = reqwest::Client::new()
        .get("https://data.btcmap.org/elements.json")
        .send()
        .await;

    if let Err(_) = response {
        log::error!("Failed to fetch response");
        std::process::exit(1);
    }

    let response = response.unwrap();
    log::info!("Fetched new data");

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
    println!("Got {} elements", fresh_elements.len());

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

    println!(
        "Of them:\nNodes {}\nWays {}\nRelations {}",
        nodes.len(),
        ways.len(),
        relations.len(),
    );

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
    println!("Found {} cached elements", elements.len());

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
            println!("Cached element with id {} was deleted from OSM", element.id);
            send_discord_message(format!(
                "{name} was deleted https://www.openstreetmap.org/{element_type}/{osm_id}"
            ))
            .await;
            let query =
                "UPDATE element SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = ?";
            println!("Executing query: {query:?}");
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
        let user = fresh_element["user"].as_str().unwrap_or("unknown user");

        match elements.iter().find(|it| it.id == btcmap_id) {
            Some(element) => {
                let element_data: String = serde_json::to_string(&element.data).unwrap();
                let fresh_element_data = serde_json::to_string(fresh_element).unwrap();

                if element_data != fresh_element_data {
                    println!("Element {btcmap_id} has been updated");
                    send_discord_message(format!(
                        "{name} was updated by {user} https://www.openstreetmap.org/{element_type}/{osm_id}"
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
                println!("Element {btcmap_id} does not exist, inserting");
                send_discord_message(format!(
                    "{name} was added by {user} https://www.openstreetmap.org/{element_type}/{osm_id}"
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
    println!("Today: {today}, year ago: {year_ago}");

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

    println!("Total elements: {}", elements.len());
    println!("Up to date elements: {}", up_to_date_elements.len());
    println!("Outdated elements: {}", outdated_elements.len());
    println!("Legacy elements: {}", legacy_elements.len());
    println!("Elements created: {elements_created}");
    println!("Elements updated: {elements_updated}");
    println!("Elements deleted: {elements_deleted}");

    let report = tx.query_row(
        db::DAILY_REPORT_SELECT_BY_DATE,
        [today.to_string()],
        db::mapper_daily_report_full(),
    );

    if let Ok(report) = report {
        println!("Found existing report, deleting");
        tx.execute(db::DAILY_REPORT_DELETE_BY_DATE, [today.to_string()])
            .unwrap();
        elements_created += report.elements_created;
        elements_updated += report.elements_updated;
        elements_deleted += report.elements_deleted;
    }

    println!("Inserting new or updated report");
    tx.execute(
        db::DAILY_REPORT_INSERT,
        params![
            today.to_string(),
            elements.len(),
            onchain_elements.len(),
            lightning_elements.len(),
            lightning_contactless_elements.len(),
            up_to_date_elements.len(),
            outdated_elements.len(),
            legacy_elements.len(),
            elements_created,
            elements_updated,
            elements_deleted
        ],
    )
    .unwrap();

    tx.commit().expect("Failed to save sync results");
    println!("Finished sync");
}

async fn send_discord_message(text: String) {
    if let Ok(discord_webhook_url) = env::var("DISCORD_WEBHOOK_URL") {
        println!("Sending Discord message");
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
                println!("Discord response status: {:?}", response.status());
            }
            Err(_) => {
                println!("Failed to send Discord message");
            }
        }
    }
}
