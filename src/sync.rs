use crate::db;
use crate::get_project_dirs;
use crate::Element;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::Statement;
use rusqlite::Transaction;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::Write;

pub async fn sync(mut db_conn: Connection) {
    println!("Starting sync");

    let project_dirs = get_project_dirs();
    let cache_dir = project_dirs.cache_dir();
    println!("Cache directory is {cache_dir:?}");

    if cache_dir.exists() {
        println!("Cache directory already exists");
    } else {
        println!("Cache directory doesn't exist, creating...");
        create_dir_all(cache_dir).expect("Failed to create cache directory");
        println!("Created cache directory {cache_dir:?}");
    }

    println!("Fetching new data...");

    let response = reqwest::Client::new()
        .get("https://data.btcmap.org/elements.json")
        .send()
        .await
        .expect("Failed to fetch new data");

    println!("Fetched new data");

    let data_file_path = cache_dir.join("elements.json");
    println!("Data file path is {data_file_path:?}");

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

    let tx: Transaction = db_conn.transaction().unwrap();
    let mut elements_stmt: Statement = tx.prepare("SELECT * FROM element").unwrap();
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

    // First, let's check if any of the cached elements no longer accept bitcoins
    for element in &elements {
        if !fresh_element_ids.contains(&element.id) && element.deleted_at.is_none() {
            println!("Cached element with {} was deleted from OSM", element.id);
            let query =
                "UPDATE element SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = ?";
            println!("Executing query: {query:?}");
            tx.execute(query, params![element.id]).unwrap();
        }
    }

    for fresh_element in fresh_elements {
        let element_type = fresh_element["type"].as_str().unwrap();
        let id = fresh_element["id"].as_i64().unwrap();
        let id = format!("{element_type}:{id}");

        match elements.iter().find(|it| it.id == id) {
            Some(element) => {
                let element_data: String = serde_json::to_string(&element.data).unwrap();
                let fresh_element_data = serde_json::to_string(fresh_element).unwrap();

                if element_data != fresh_element_data {
                    println!("Element {id} has been changed");

                    tx.execute(
                        "UPDATE element SET data = ? WHERE id = ?",
                        params![fresh_element_data, id],
                    )
                    .unwrap();
                }
            }
            None => {
                println!("Element {id} does not exist, inserting");

                tx.execute(
                    "INSERT INTO element (id, data) VALUES (?, ?)",
                    params![id, serde_json::to_string(fresh_element).unwrap()],
                )
                .unwrap();
            }
        }
    }

    tx.commit().expect("Failed to save sync results");
    println!("Finished sync");
}
