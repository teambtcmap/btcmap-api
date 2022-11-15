use actix_web::middleware::Compress;
use actix_web::middleware::NormalizePath;
use actix_web::web::scope;
use actix_web::web::Data;
extern crate core;

mod auth;
mod controller;
mod db;
mod error;
mod generate_android_icons;
mod generate_element_categories;
mod generate_report;
mod model;
mod pouch;
mod sync;
mod sync_users;

use crate::error::Error;

use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;

use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use directories::ProjectDirs;
use rusqlite::Connection;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    if env::var("ADMIN_TOKEN").is_err() && cfg!(debug_assertions) {
        env::set_var("ADMIN_TOKEN", "debug");
    }

    env_logger::init();

    if env::var("RUST_BACKTRACE").is_err() {
        log::info!("Activating RUST_BACKTRACE");
        env::set_var("RUST_BACKTRACE", "1");
    }

    let mut db = open_db_connection()?;
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            db::migrate(&mut db)?;

            log::info!("Starting HTTP server");
            HttpServer::new(move || {
                App::new()
                    .wrap(Logger::default())
                    .wrap(NormalizePath::trim())
                    .wrap(Compress::default())
                    .app_data(Data::new(open_db_connection().unwrap()))
                    .service(
                        scope("elements")
                            .service(controller::element_v2::get)
                            .service(controller::element_v2::get_by_id)
                            .service(controller::element_v2::post_tags),
                    )
                    .service(
                        scope("events")
                            .service(controller::event_v2::get)
                            .service(controller::event_v2::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(controller::user_v2::get)
                            .service(controller::user_v2::get_by_id)
                            .service(controller::user_v2::post_tags),
                    )
                    .service(
                        scope("areas")
                            .service(controller::area_v2::post)
                            .service(controller::area_v2::get)
                            .service(controller::area_v2::get_by_id)
                            .service(controller::area_v2::post_tags),
                    )
                    .service(
                        scope("reports")
                            .service(controller::report_v2::get)
                            .service(controller::report_v2::get_by_id),
                    )
                    .service(
                        scope("v2")
                            .service(
                                scope("elements")
                                    .service(controller::element_v2::get)
                                    .service(controller::element_v2::get_by_id)
                                    .service(controller::element_v2::post_tags),
                            )
                            .service(
                                scope("events")
                                    .service(controller::event_v2::get)
                                    .service(controller::event_v2::get_by_id),
                            )
                            .service(
                                scope("users")
                                    .service(controller::user_v2::get)
                                    .service(controller::user_v2::get_by_id)
                                    .service(controller::user_v2::post_tags),
                            )
                            .service(
                                scope("areas")
                                    .service(controller::area_v2::post)
                                    .service(controller::area_v2::get)
                                    .service(controller::area_v2::get_by_id)
                                    .service(controller::area_v2::post_tags),
                            )
                            .service(
                                scope("reports")
                                    .service(controller::report_v2::get)
                                    .service(controller::report_v2::get_by_id),
                            ),
                    )
            })
            .bind(("127.0.0.1", 8000))?
            .run()
            .await
        }
        _ => {
            match args.get(1).unwrap().as_str() {
                "db" => {
                    db::cli_main(&args[2..], db).unwrap();
                }
                "sync" => {
                    sync::sync(db).await;
                }
                "sync-users" => {
                    sync_users::sync(db).await;
                }
                "generate-report" => {
                    generate_report::generate_report(db).await;
                }
                "generate-android-icons" => {
                    generate_android_icons::generate_android_icons(db).await;
                }
                "generate-element-categories" => {
                    generate_element_categories::generate_element_categories(db).await;
                }
                "pouch" => {
                    pouch::pouch(db).await;
                }
                _ => {
                    log::error!("Unknown action");
                    std::process::exit(1);
                }
            }

            Ok(())
        }
    }
}

fn open_db_connection() -> Result<Connection> {
    let conn = Connection::open(get_db_file_path()?)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(conn)
}

fn get_db_file_path() -> Result<PathBuf, std::io::Error> {
    let project_dirs = match ProjectDirs::from("org", "BTC Map", "BTC Map") {
        Some(project_dirs) => project_dirs,
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Can't find home directory",
        ))?,
    };

    if !project_dirs.data_dir().exists() {
        create_dir_all(project_dirs.data_dir())?;
    }

    Ok(project_dirs.data_dir().join("btcmap.db"))
}
