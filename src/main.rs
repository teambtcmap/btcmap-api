extern crate core;
use command::db;
use command::generate_android_icons;
use command::generate_element_categories;
use command::generate_reports;
mod server;
pub use error::Error;
mod auth;
mod command;
mod discord;
mod element;
mod error;
mod event;
mod osm;
mod report;
#[cfg(test)]
mod test;
mod tile;
mod user;
use rusqlite::Connection;
use std::env;
use std::process::ExitCode;
use tracing::error;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod area;
mod review;
mod rpc;
mod vacuum;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> ExitCode {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(Layer::new().json())
        .init();

    let mut conn = match command::db::open_connection() {
        Ok(v) => v,
        Err(e) => {
            error!(?e, "Failed to open database connection");
            return ExitCode::FAILURE;
        }
    };

    let args: Vec<String> = env::args().collect();

    match args.get(1).unwrap_or(&"".into()).as_str() {
        "server" => {
            if let Err(e) = db::migrate(&mut conn) {
                error!(?e, "Failed to open database connection");
                return ExitCode::FAILURE;
            }

            if let Err(e) = server::run().await {
                error!(?e, "Failed to start a server");
                return ExitCode::FAILURE;
            }
        }
        "sync" => {
            if let Err(e) = command::sync::run(conn).await {
                error!(?e, "Failed to sync elements");
                return ExitCode::FAILURE;
            }
        }
        "generate-reports" => {
            if let Err(e) = generate_reports::run(&mut conn) {
                error!(?e, "Failed to generate reports");
                return ExitCode::FAILURE;
            }
        }
        "generate-android-icons" => {
            if let Err(e) = generate_android_icons::run(&conn).await {
                error!(?e, "Failed to generate Android icons");
                return ExitCode::FAILURE;
            }
        }
        "generate-element-categories" => {
            if let Err(e) = generate_element_categories::run(&conn).await {
                error!(?e, "Failed to generate element categories");
                return ExitCode::FAILURE;
            }
        }
        "update-areas-tag" => {
            if let Err(e) = command::update_areas_tag::run(args) {
                error!(?e, "Failed to add areas tag");
                return ExitCode::FAILURE;
            }
        }
        "remove-areas-tag" => {
            if let Err(e) = command::remove_areas_tag::run(args) {
                error!(?e, "Failed to remove areas tag");
                return ExitCode::FAILURE;
            }
        }
        "vacuum" => {
            if let Err(e) = vacuum::vacuum_areas(&conn) {
                error!(?e, "Failed to vacuum database");
                return ExitCode::FAILURE;
            }
        }
        first_arg => {
            error!(command = first_arg, "Unknown command");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
