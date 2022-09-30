use crate::area::Area;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use std::ops::Sub;
extern crate core;

mod area;
mod daily_report;
mod db;
mod element;
mod sync;

use crate::daily_report::DailyReport;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Mutex;
use time::Duration;
use time::OffsetDateTime;

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
    let mut db_conn = Connection::open(get_db_file_path()).unwrap();

    match args.len() {
        1 => {
            if let Err(err) = db::migrate(&mut db_conn) {
                eprintln!("Migration faied: {err}");
                std::process::exit(1);
            }

            let db_conn = web::Data::new(Mutex::new(db_conn));

            println!("Starting HTTP server");
            HttpServer::new(move || {
                App::new()
                    .wrap(Logger::default())
                    .app_data(db_conn.clone())
                    .service(get_elements)
                    .service(get_element)
                    .service(get_daily_reports)
                    .service(get_areas)
                    .service(get_area)
                    .service(get_area_elements)
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

#[derive(serde::Serialize, Debug)]
struct ApiError {
    message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::convert::From<rusqlite::Error> for ApiError {
    fn from(error: rusqlite::Error) -> Self {
        ApiError {
            message: error.to_string(),
        }
    }
}

impl std::convert::From<std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>>
    for ApiError
{
    fn from(_: std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>) -> Self {
        ApiError {
            message: "Failed to lock database connection".to_string(),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(500).unwrap())
            .insert_header(ContentType::json())
            .body(serde_json::to_string(self).unwrap())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(500).unwrap()
    }
}

#[derive(serde::Deserialize)]
struct GetPlacesArgs {
    updated_since: Option<String>,
}

#[actix_web::get("/elements")]
async fn get_elements(
    args: web::Query<GetPlacesArgs>,
    conn: web::Data<Mutex<Connection>>,
) -> Result<Json<Vec<Element>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_ALL)?
            .query_map([], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
    }))
}

#[actix_web::get("/elements/{id}")]
async fn get_element(
    path: web::Path<String>,
    conn: web::Data<Mutex<Connection>>,
) -> Result<Json<Option<Element>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::ELEMENT_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_element_full(),
            )
            .optional()?,
    ))
}

#[actix_web::get("/daily_reports")]
async fn get_daily_reports(conn: web::Data<Mutex<Connection>>) -> Json<Vec<DailyReport>> {
    let conn = conn.lock().unwrap();
    let query = "SELECT * FROM daily_report ORDER BY date DESC";
    let mut stmt: Statement = conn.prepare(query).unwrap();
    let reports: Vec<DailyReport> = stmt
        .query_map([], db::mapper_daily_report_full())
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    Json(reports)
}

#[derive(Serialize, Deserialize)]
pub struct GetAreasItem {
    pub id: String,
    pub name: String,
    pub area_type: String,
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
    pub elements: usize,
    pub up_to_date_elements: usize,
}

#[actix_web::get("/areas")]
async fn get_areas(conn: web::Data<Mutex<Connection>>) -> Json<Vec<GetAreasItem>> {
    let conn = conn.lock().unwrap();

    let query = "SELECT * FROM area ORDER BY name";
    let mut stmt: Statement = conn.prepare(query).unwrap();

    let areas: Vec<Area> = stmt
        .query_map([], db::mapper_area_full())
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    let query = "SELECT * FROM element ORDER BY updated_at DESC";
    let mut stmt: Statement = conn.prepare(query).unwrap();

    let elements: Vec<Element> = stmt
        .query_map([], db::mapper_element_full())
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    let mut res: Vec<GetAreasItem> = vec![];
    let today = OffsetDateTime::now_utc().date();
    let year_ago = today.sub(Duration::days(365));

    for area in areas {
        let area_elements: Vec<&Element> = elements
            .iter()
            .filter(|it| it.data["type"].as_str().unwrap() == "node")
            .filter(|it| {
                let lat = it.data["lat"].as_f64().unwrap();
                let lon = it.data["lon"].as_f64().unwrap();
                lon > area.min_lon && lon < area.max_lon && lat > area.min_lat && lat < area.max_lat
            })
            .collect();

        let elements_len = area_elements.len();

        let up_to_date_elements: Vec<&Element> = area_elements
            .into_iter()
            .filter(|it| {
                (it.data["tags"].get("survey:date").is_some()
                    && it.data["tags"]["survey:date"].as_str().unwrap().to_string()
                        > year_ago.to_string())
                    || (it.data["tags"].get("check_date").is_some()
                        && it.data["tags"]["check_date"].as_str().unwrap().to_string()
                            > year_ago.to_string())
            })
            .collect();

        res.push(GetAreasItem {
            id: area.id,
            name: area.name,
            area_type: area.area_type,
            min_lon: area.min_lon,
            min_lat: area.min_lat,
            max_lon: area.max_lon,
            max_lat: area.max_lat,
            elements: elements_len,
            up_to_date_elements: up_to_date_elements.len(),
        });
    }

    Json(res)
}

#[actix_web::get("/areas/{id}")]
async fn get_area(
    path: web::Path<String>,
    conn: web::Data<Mutex<Connection>>,
) -> Json<Option<GetAreasItem>> {
    let id = path.into_inner();
    let conn = conn.lock().unwrap();

    let query = "SELECT * FROM area WHERE id = ?";
    let area = conn
        .query_row(query, [id], db::mapper_area_full())
        .optional()
        .unwrap()
        .map(|area| {
            let query = "SELECT * FROM element ORDER BY updated_at DESC";
            let mut stmt: Statement = conn.prepare(query).unwrap();
            let elements: Vec<Element> = stmt
                .query_map([], db::mapper_element_full())
                .unwrap()
                .map(|row| row.unwrap())
                .collect();

            let area_elements: Vec<&Element> = elements
                .iter()
                .filter(|it| it.data["type"].as_str().unwrap() == "node")
                .filter(|it| {
                    let lat = it.data["lat"].as_f64().unwrap();
                    let lon = it.data["lon"].as_f64().unwrap();
                    lon > area.min_lon
                        && lon < area.max_lon
                        && lat > area.min_lat
                        && lat < area.max_lat
                })
                .collect();

            let elements_len = area_elements.len();
            let today = OffsetDateTime::now_utc().date();
            let year_ago = today.sub(Duration::days(365));

            let up_to_date_elements: Vec<&Element> = area_elements
                .into_iter()
                .filter(|it| {
                    (it.data["tags"].get("survey:date").is_some()
                        && it.data["tags"]["survey:date"].as_str().unwrap().to_string()
                            > year_ago.to_string())
                        || (it.data["tags"].get("check_date").is_some()
                            && it.data["tags"]["check_date"].as_str().unwrap().to_string()
                                > year_ago.to_string())
                })
                .collect();

            GetAreasItem {
                id: area.id,
                name: area.name,
                area_type: area.area_type,
                min_lon: area.min_lon,
                min_lat: area.min_lat,
                max_lon: area.max_lon,
                max_lat: area.max_lat,
                elements: elements_len,
                up_to_date_elements: up_to_date_elements.len(),
            }
        });

    Json(area)
}

#[actix_web::get("/areas/{id}/elements")]
async fn get_area_elements(
    path: web::Path<String>,
    conn: web::Data<Mutex<Connection>>,
) -> Json<Vec<Element>> {
    let id = path.into_inner();
    let conn = conn.lock().unwrap();

    let query = "SELECT * FROM area WHERE id = ?";

    let area = conn
        .query_row(query, [id], db::mapper_area_full())
        .optional()
        .unwrap();

    if let None = area {
        return Json(vec![]);
    }

    let area = area.unwrap();

    let query = "SELECT * FROM element ORDER BY updated_at DESC";
    let mut stmt: Statement = conn.prepare(query).unwrap();

    let elements: Vec<Element> = stmt
        .query_map([], db::mapper_element_full())
        .unwrap()
        .map(|row| row.unwrap())
        .filter(|it| {
            let element_type = it.data["type"].as_str().unwrap();

            if element_type != "node" {
                return false;
            }

            let lat = it.data["lat"].as_f64().unwrap();
            let lon = it.data["lon"].as_f64().unwrap();

            lon > area.min_lon && lon < area.max_lon && lat > area.min_lat && lat < area.max_lat
        })
        .collect();

    Json(elements)
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
