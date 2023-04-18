#![feature(exclusive_range_pattern)]

extern crate core;

pub use error::ApiError;
pub use error::Error;
mod command;
mod controller;
mod error;
mod model;
mod service;
use rusqlite::Connection;
use std::env;
use std::process::ExitCode;
use tracing::error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();

    let mut db = match command::db::open_connection() {
        Ok(v) => v,
        Err(e) => {
            error!(?e, "Failed to open database connection");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = command::db::migrate(&mut db) {
        error!(?e, "Failed to open database connection");
        return ExitCode::FAILURE;
    }

    let args: Vec<String> = env::args().collect();

    let command = match args.get(1) {
        Some(v) => v,
        None => {
            error!("No actions passed");
            return ExitCode::FAILURE;
        }
    };

    match command.as_str() {
        "server" => {
            if let Err(e) = command::server::run().await {
                error!(?e, "Failed to start a server");
                return ExitCode::FAILURE;
            }
        }
        "db" => {
            if let Err(e) = command::db::run(&args[2..], db) {
                error!(?e, "Failed execute database action");
                return ExitCode::FAILURE;
            }
        }
        "sync" => {
            if let Err(e) = command::sync::run(db).await {
                error!(?e, "Failed to sync elements");
                return ExitCode::FAILURE;
            }
        }
        "sync-users" => {
            if let Err(e) = command::sync_users::run(db).await {
                error!(?e, "Failed to sync users");
                return ExitCode::FAILURE;
            }
        }
        "generate-report" => {
            if let Err(e) = command::generate_report::run(db).await {
                error!(?e, "Failed to generate reports");
                return ExitCode::FAILURE;
            }
        }
        "generate-android-icons" => {
            if let Err(e) = command::generate_android_icons::run(db).await {
                error!(?e, "Failed to generate Android icons");
                return ExitCode::FAILURE;
            }
        }
        "generate-element-categories" => {
            if let Err(e) = command::generate_element_categories::run(db).await {
                error!(?e, "Failed to generate element categories");
                return ExitCode::FAILURE;
            }
        }
        "lint" => {
            if let Err(e) = command::lint::run(db).await {
                error!(?e, "Failed to run linter");
                return ExitCode::FAILURE;
            }
        }
        "fetch-areas" => {
            if let Err(e) = command::fetch_areas::run(db, args[2].clone()).await {
                error!(?e, "Failed to fetch areas");
                return ExitCode::FAILURE;
            }
        }
        "analyze-logs" => {
            if let Err(e) = command::analyze_logs::run().await {
                error!(?e, "Failed to analyze logs");
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

fn init_logging() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt().json().init();
}
