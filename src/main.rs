#![feature(exclusive_range_pattern)]

extern crate core;
use command::add_area;
use command::analyze_logs;
use command::db;
use command::db::pool;
use command::fix_tags;
use command::generate_android_icons;
use command::generate_element_categories;
use command::generate_reports;
use command::import_countries;
use command::lint;
use command::server;
use command::sync;
use command::sync_users;
use discord::DiscordLayer;
pub use error::ApiError;
pub use error::Error;
mod command;
mod controller;
mod discord;
mod error;
mod model;
mod service;
#[cfg(test)]
mod test;
use area::AreaRepo;
use rusqlite::Connection;
use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use tracing::error;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod area;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> ExitCode {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(Layer::new().json())
        .with(DiscordLayer)
        .init();

    let mut db = match command::db::open_connection() {
        Ok(v) => v,
        Err(e) => {
            error!(?e, "Failed to open database connection");
            return ExitCode::FAILURE;
        }
    };

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
            if let Err(e) = db::migrate(&mut db) {
                error!(?e, "Failed to open database connection");
                return ExitCode::FAILURE;
            }

            if let Err(e) = server::run().await {
                error!(?e, "Failed to start a server");
                return ExitCode::FAILURE;
            }
        }
        "db" => {
            if let Err(e) = db::run(&args[2..], db) {
                error!(?e, "Failed execute database action");
                return ExitCode::FAILURE;
            }
        }
        "sync" => {
            if let Err(e) = sync::run(db).await {
                error!(?e, "Failed to sync elements");
                return ExitCode::FAILURE;
            }
        }
        "sync-users" => {
            if let Err(e) = sync_users::run(db).await {
                error!(?e, "Failed to sync users");
                return ExitCode::FAILURE;
            }
        }
        "generate-report" => {
            if let Err(e) = generate_reports::run(db).await {
                error!(?e, "Failed to generate reports");
                return ExitCode::FAILURE;
            }
        }
        "generate-reports" => {
            if let Err(e) = generate_reports::run(db).await {
                error!(?e, "Failed to generate reports");
                return ExitCode::FAILURE;
            }
        }
        "generate-android-icons" => {
            if let Err(e) = generate_android_icons::run(&db).await {
                error!(?e, "Failed to generate Android icons");
                return ExitCode::FAILURE;
            }
        }
        "generate-element-categories" => {
            if let Err(e) = generate_element_categories::run(&db).await {
                error!(?e, "Failed to generate element categories");
                return ExitCode::FAILURE;
            }
        }
        "lint" => {
            if let Err(e) = lint::run(db).await {
                error!(?e, "Failed to run linter");
                return ExitCode::FAILURE;
            }
        }
        "analyze-logs" => {
            if let Err(e) = analyze_logs::run().await {
                error!(?e, "Failed to analyze logs");
                return ExitCode::FAILURE;
            }
        }
        "add-area" => {
            let repo = AreaRepo::new(Arc::new(pool().unwrap()));
            if let Err(e) = add_area::run(&repo).await {
                error!(?e, "Failed to add area");
                return ExitCode::FAILURE;
            }
        }
        "import-countries" => {
            let repo = AreaRepo::new(Arc::new(pool().unwrap()));
            if let Err(e) = import_countries::run(args.get(2).unwrap_or(&"".into()), &repo).await {
                error!(?e, "Failed to import countries");
                return ExitCode::FAILURE;
            }
        }
        "fix-tags" => {
            let repo = AreaRepo::new(Arc::new(pool().unwrap()));
            if let Err(e) = fix_tags::run(&repo).await {
                error!(?e, "Failed to fix tags");
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
