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

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let mut db = command::db::open_connection()?;

    command::db::migrate(&mut db)?;

    let args: Vec<String> = env::args().collect();

    let command = match args.get(1) {
        Some(some) => some,
        None => Err(Error::CLI("No actions passed".into()))?,
    };

    match command.as_str() {
        "server" => command::server::run().await?,
        "db" => command::db::run(&args[2..], db)?,
        "sync" => command::sync::run(db).await?,
        "sync-users" => command::sync_users::run(db).await?,
        "generate-report" => command::generate_report::run(db).await?,
        "generate-android-icons" => command::generate_android_icons::run(db).await?,
        "generate-element-categories" => command::generate_element_categories::run(db).await?,
        "lint" => command::lint::run(db).await?,
        "fetch-areas" => command::fetch_areas::run(db, args[2].clone()).await?,
        "analyze-logs" => command::analyze_logs::run().await?,
        first_arg => Err(Error::CLI(format!("Unknown command: {first_arg}")))?,
    }

    Ok(())
}

fn init_logging() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    if cfg!(debug_assertions) {
        env_logger::init();
    } else {
        env_logger::builder().format_timestamp(None).init();
    }
}
