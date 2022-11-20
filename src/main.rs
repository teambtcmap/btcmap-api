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
async fn main() {
    init_logging();

    let mut db = match command::db::open_connection() {
        Ok(ok) => ok,
        Err(e) => {
            log::error!("Failed to connect to database: {e}");
            return;
        }
    };

    if let Err(e) = command::db::migrate(&mut db) {
        log::error!("Migration failed: {e}");
        return;
    }

    let args: Vec<String> = env::args().collect();

    if let Err(e) = _main(&args[1..], db).await {
        log::error!("{e}");
    }
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

// TODO consider replacing with try block
async fn _main(args: &[String], db: Connection) -> Result<()> {
    let first_arg = match args.first() {
        Some(some) => some,
        None => Err(Error::CLI("No actions passed".into()))?,
    };

    match first_arg.as_str() {
        "server" => command::server::run().await,
        "db" => command::db::run(&args[1..], db),
        "sync" => command::sync::run(db).await,
        "sync-users" => command::sync_users::run(db).await,
        "generate-report" => command::generate_report::run(db).await,
        "generate-android-icons" => command::generate_android_icons::run(db).await,
        "generate-element-categories" => command::generate_element_categories::run(db).await,
        "fetch-pouch-tags" => command::fetch_pouch_tags::run(db).await,
        first_arg => Err(Error::CLI(format!("Unknown action: {first_arg}"))),
    }
}
