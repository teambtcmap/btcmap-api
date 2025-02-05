use actix_governor::{Governor, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use actix_web::middleware::{from_fn, Compress, ErrorHandlers, NormalizePath};
use actix_web::{App, HttpServer};
use conf::Conf;
use error::Error;
mod admin;
mod conf;
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
use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod area;
mod area_element;
mod boost;
mod db;
mod element_comment;
mod feed;
mod invoice;
mod log;
mod rpc;
mod sync;
use actix_web::http::header::HeaderValue;
use actix_web::web::{scope, Data, QueryConfig};
mod ban;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(Layer::new().json())
        .init();

    // All the worker threads share a single connection pool
    let pool = Arc::new(db::pool()?);

    pool.get().await?.interact(db::migrate).await??;

    let conf = Arc::new(Conf::select_async(&pool).await?);

    let rate_limit_conf = GovernorConfigBuilder::default()
        .milliseconds_per_request(500)
        .burst_size(50)
        .key_extractor(get_key_extractor())
        .finish()
        .unwrap();

    let tile_rate_limit_conf = GovernorConfigBuilder::default()
        .milliseconds_per_request(500)
        .burst_size(1000)
        .key_extractor(get_key_extractor())
        .finish()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(from_fn(log::middleware))
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .app_data(Data::from(pool.clone()))
            .app_data(Data::from(conf.clone()))
            .app_data(QueryConfig::default().error_handler(error::query_error_handler))
            .service(
                scope("rpc")
                    .wrap(ErrorHandlers::new().default_handler(rpc::handler::handle_rpc_error))
                    .service(rpc::handler::handle),
            )
            .service(
                scope("tiles")
                    .wrap(Governor::new(&tile_rate_limit_conf))
                    .service(tile::controller::get),
            )
            .service(
                scope("feeds")
                    .service(feed::atom::new_places)
                    .service(feed::atom::new_places_for_area)
                    .service(feed::atom::new_comments)
                    .service(feed::atom::new_comments_for_area),
            )
            .service(
                scope("v2")
                    .wrap(Governor::new(&rate_limit_conf))
                    .wrap(from_fn(ban::check_if_banned))
                    .service(
                        scope("elements")
                            .service(element::v2::get)
                            .service(element::v2::get_by_id),
                    )
                    .service(
                        scope("events")
                            .service(event::v2::get)
                            .service(event::v2::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(user::v2::get)
                            .service(user::v2::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(area::v2::get)
                            .service(area::v2::get_by_url_alias),
                    )
                    .service(
                        scope("reports")
                            .service(report::v2::get)
                            .service(report::v2::get_by_id),
                    ),
            )
            .service(
                scope("v3")
                    .wrap(Governor::new(&rate_limit_conf))
                    .service(
                        scope("elements")
                            .service(element::v3::get)
                            .service(element::v3::get_by_id),
                    )
                    .service(
                        scope("element-comments")
                            .service(element_comment::v3::get)
                            .service(element_comment::v3::get_by_id),
                    )
                    .service(
                        scope("events")
                            .service(event::v3::get)
                            .service(event::v3::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(area::v3::get)
                            .service(area::v3::get_by_id),
                    )
                    .service(
                        scope("reports")
                            .service(report::v3::get)
                            .service(report::v3::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(user::v3::get)
                            .service(user::v3::get_by_id),
                    )
                    .service(
                        scope("area-elements")
                            .service(area_element::v3::get)
                            .service(area_element::v3::get_by_id),
                    ),
            )
            .service(
                scope("v4").wrap(Governor::new(&rate_limit_conf)).service(
                    scope("elements")
                        .service(element::v4::get)
                        .service(element::v4::get_by_id),
                ),
            )
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}

pub fn data_dir_file(name: &str) -> Result<PathBuf> {
    #[allow(deprecated)]
    let data_dir = std::env::home_dir()
        .ok_or("Home directory does not exist")?
        .join(".local/share/btcmap");
    if !data_dir.exists() {
        create_dir_all(&data_dir)?;
    }
    Ok(data_dir.join(name))
}

#[cfg(not(debug_assertions))]
pub fn get_key_extractor() -> RealIpKeyExtractor {
    RealIpKeyExtractor
}

#[cfg(debug_assertions)]
pub fn get_key_extractor() -> actix_governor::PeerIpKeyExtractor {
    actix_governor::PeerIpKeyExtractor
}

#[derive(Clone)]
pub struct RealIpKeyExtractor;

impl KeyExtractor for RealIpKeyExtractor {
    type Key = HeaderValue;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        req.headers()
            .get("x-forwarded-for")
            .cloned()
            .ok_or_else(|| {
                SimpleKeyExtractionError::new("Could not extract real IP address from request")
            })
    }
}
