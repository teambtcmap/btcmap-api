extern crate core;

mod db;
mod element;
mod sync;

use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::element::Element;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::web::Json;
use actix_web::{App, HttpServer};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Statement};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if env::var("RUST_BACKTRACE").is_err() {
        env::set_var("RUST_BACKTRACE", "1");
    }

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let db_conn = Connection::open(get_db_file_path()).unwrap();

    match args.len() {
        1 => {
            let db_conn = web::Data::new(Mutex::new(db_conn));

            println!("Starting HTTP server");
            HttpServer::new(move || {
                App::new()
                    .wrap(Logger::default())
                    .app_data(db_conn.clone())
                    .service(get_elements)
                    .service(get_element)
            })
            .bind(("127.0.0.1", 8000))?
            .run()
            .await
        }
        _ => {
            let db_conn = Connection::open(get_db_file_path()).unwrap();

            match args.get(1).unwrap().as_str() {
                "db" => {
                    db::cli_main(&args[2..], db_conn);
                }
                "sync" => {
                    sync::sync(db_conn).await;
                }
                _ => {
                    panic!("Unknown action");
                }
            }

            Ok(())
        }
    }
}

#[derive(serde::Deserialize)]
struct GetPlacesArgs {
    created_or_updated_since: Option<String>,
}

#[actix_web::get("/elements")]
async fn get_elements(
    args: web::Query<GetPlacesArgs>,
    conn: web::Data<Mutex<Connection>>,
) -> Json<Vec<Element>> {
    let conn = conn.lock().unwrap();

    let places: Vec<Element> = match &args.created_or_updated_since {
        Some(created_or_updated_since) => {
            let query = "SELECT * FROM element WHERE updated_at > ? ORDER BY updated_at DESC";
            let mut stmt: Statement = conn.prepare(query).unwrap();
            stmt.query_map([created_or_updated_since], db::mapper_element_full())
                .unwrap()
                .map(|row| row.unwrap())
                .collect()
        }
        None => {
            let query = "SELECT * FROM element ORDER BY updated_at DESC";
            let mut stmt: Statement = conn.prepare(query).unwrap();
            stmt.query_map([], db::mapper_element_full())
                .unwrap()
                .map(|row| row.unwrap())
                .collect()
        }
    };

    Json(places)
}

#[actix_web::get("/elements/{id}")]
async fn get_element(
    path: web::Path<String>,
    conn: web::Data<Mutex<Connection>>,
) -> Json<Option<Element>> {
    let id = path.into_inner();

    let query = "SELECT * FROM element WHERE id = ?";
    let place = conn
        .lock()
        .unwrap()
        .query_row(query, [id], db::mapper_element_full())
        .optional()
        .unwrap();

    Json(place)
}

fn get_db_file_path() -> PathBuf {
    let project_dirs = get_project_dirs();

    if !project_dirs.data_dir().exists() {
        create_dir_all(project_dirs.data_dir()).unwrap()
    }

    project_dirs.data_dir().join("btcmap.db")
}

fn get_project_dirs() -> ProjectDirs {
    return ProjectDirs::from("org", "BTC Map", "BTC Map").unwrap();
}
