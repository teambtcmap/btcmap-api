use actix_web::error::InternalError;
use actix_web::middleware::{from_fn, Compress, ErrorHandlers, NormalizePath};
use actix_web::{web, App, HttpServer, ResponseError};
use error::Error;
use rest::error::{RestApiError, RestApiErrorCode};
mod error;
use std::env;
use std::time::Duration;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod feed;
mod rpc;
use crate::service::log::Log;
use actix_web::web::{scope, Data};
mod db;
mod og;
mod rest;
mod service;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> Result<()> {
    init_env();

    let main_pool = db::main::pool()?;
    let image_pool = db::image::pool()?;
    let log_pool = db::log::pool()?;

    let conf = db::main::conf::queries::select(&main_pool).await?;

    check_areas_without_icon_square(&main_pool).await;

    service::matrix::init(&main_pool);

    HttpServer::new(move || {
        App::new()
            .wrap(Log)
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .wrap(from_fn(service::ban::check_if_banned))
            .app_data(Data::new(main_pool.clone()))
            .app_data(Data::new(image_pool.clone()))
            .app_data(Data::new(log_pool.clone()))
            .app_data(Data::new(conf.clone()))
            .service(og::element::get_element)
            .service(
                scope("rpc")
                    .app_data(Data::new(log_pool.clone()))
                    .wrap(ErrorHandlers::new().default_handler(rpc::handler::handle_rpc_error))
                    .service(rpc::handler::handle),
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
                    .service(
                        scope("elements")
                            .service(rest::v2::elements::get)
                            .service(rest::v2::elements::get_by_id),
                    )
                    .service(
                        scope("events")
                            .service(rest::v2::events::get)
                            .service(rest::v2::events::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(rest::v2::users::get)
                            .service(rest::v2::users::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(rest::v2::areas::get)
                            .service(rest::v2::areas::get_by_url_alias),
                    )
                    .service(
                        scope("reports")
                            .service(rest::v2::reports::get)
                            .service(rest::v2::reports::get_by_id),
                    ),
            )
            .service(
                scope("v3")
                    .service(
                        scope("elements")
                            .service(rest::v3::elements::get)
                            .service(rest::v3::elements::get_by_id),
                    )
                    .service(
                        scope("element-comments")
                            .service(rest::v3::element_comments::get)
                            .service(rest::v3::element_comments::get_by_id),
                    )
                    .service(
                        scope("events")
                            .service(rest::v3::events::get)
                            .service(rest::v3::events::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(rest::v3::areas::get)
                            .service(rest::v3::areas::get_by_id),
                    )
                    .service(
                        scope("reports")
                            .service(rest::v3::reports::get)
                            .service(rest::v3::reports::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(rest::v3::users::get)
                            .service(rest::v3::users::get_by_id),
                    )
                    .service(
                        scope("area-elements")
                            .service(rest::v3::area_elements::get)
                            .service(rest::v3::area_elements::get_by_id),
                    ),
            )
            .service(
                scope("v4")
                    .configure(|cfg| {
                        cfg.app_data(web::QueryConfig::default().error_handler(|err, _req| {
                            InternalError::from_response(
                                err,
                                RestApiError::new(
                                    RestApiErrorCode::InvalidInput,
                                    "Invalid query parameters",
                                )
                                .error_response(),
                            )
                            .into()
                        }));
                    })
                    .service(
                        scope("places")
                            .service(rest::v4::places::get_saved)
                            .service(rest::v4::places::put_saved)
                            .service(rest::v4::places::get)
                            .service(rest::v4::places::get_pending)
                            .service(rest::v4::places::search)
                            .service(rest::v4::places::get_by_id)
                            .service(rest::v4::places::get_by_id_comments)
                            .service(rest::v4::places::get_by_id_areas)
                            .service(rest::v4::places::get_by_id_activity),
                    )
                    .service(scope("invoices").service(rest::v4::invoices::get_by_id))
                    .service(
                        scope("events")
                            .service(rest::v4::events::get)
                            .service(rest::v4::events::get_by_id),
                    )
                    .service(
                        scope("place-issues")
                            .service(rest::v4::element_issues::get)
                            .service(rest::v4::element_issues::get_by_id),
                    )
                    .service(
                        scope("place-comments")
                            .service(rest::v4::place_comments::get)
                            .service(rest::v4::place_comments::get_quote)
                            .service(rest::v4::place_comments::get_by_id)
                            .service(rest::v4::place_comments::post),
                    )
                    .service(
                        scope("place-boosts")
                            .service(rest::v4::place_boosts::get_quote)
                            .service(rest::v4::place_boosts::post),
                    )
                    .service(scope("search").service(rest::v4::search::get))
                    .service(
                        scope("areas")
                            .service(rest::v4::areas::get_saved)
                            .service(rest::v4::areas::put_saved)
                            .service(rest::v4::areas::get),
                    )
                    .service(scope("dashboard").service(rest::v4::dashboard::get))
                    .service(scope("top-editors").service(rest::v4::top_editors::get))
                    .service(scope("communities").service(rest::v4::communities::get_top))
                    .service(scope("countries").service(rest::v4::countries::get_top))
                    .service(scope("activity").service(rest::v4::activity::get))
                    .service(
                        scope("users")
                            .service(rest::v4::users::me)
                            .service(rest::v4::users::post)
                            .service(rest::v4::users::create_token),
                    ),
            )
    })
    .client_request_timeout(Duration::from_millis(0))
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}

fn init_env() {
    if env::var("RUST_LOG").is_err() {
        unsafe {
            env::set_var("RUST_LOG", "info");
        }
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(Layer::new().json())
        .init();
}

async fn check_areas_without_icon_square(pool: &deadpool_sqlite::Pool) {
    use crate::db::main::area::queries;
    use reqwest::Client;
    use serde_json::Map;
    use serde_json::Value;

    match queries::select_without_icon_square(pool).await {
        Ok(areas) => {
            if areas.is_empty() {
                tracing::warn!("All non-deleted areas have icon:square tag");
                return;
            }

            let names: Vec<_> = areas.iter().map(|a| a.name()).collect();
            tracing::warn!(
                "Found {} non-deleted areas without icon:square tag: {:?}",
                areas.len(),
                names
            );

            let client = Client::new();

            for area in areas {
                let alias = area.alias();
                tracing::warn!("Checking area '{}' with alias '{}'", area.name(), alias);

                if alias.len() != 2 || !alias.chars().all(|c| c.is_ascii_lowercase()) {
                    tracing::warn!(
                        "Skipping area '{}': alias '{}' is not a two-letter lowercase code",
                        area.name(),
                        alias
                    );
                    continue;
                }

                let url = format!("https://static.btcmap.org/images/countries/{}.svg", alias);
                tracing::warn!("Checking URL: {}", url);

                match client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status();
                        tracing::warn!("URL {} returned status {}", url, status);

                        if !status.is_success() {
                            tracing::warn!(
                                "Skipping area '{}': URL {} returned non-success status {}",
                                area.name(),
                                url,
                                status
                            );
                            continue;
                        }

                        let content_type = response
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        tracing::warn!("Content-Type: {}", content_type);

                        if !content_type.contains("svg") {
                            tracing::warn!(
                                "Skipping area '{}': URL {} returned content-type '{}' instead of SVG",
                                area.name(),
                                url,
                                content_type
                            );
                            continue;
                        }

                        tracing::warn!(
                            "Saving icon:square URL '{}' for area '{}'",
                            url,
                            area.name()
                        );

                        let mut tags = Map::new();
                        tags.insert("icon:square".to_string(), Value::String(url.clone()));

                        match queries::patch_tags(area.id, tags, pool).await {
                            Ok(_) => {
                                tracing::warn!(
                                    "Successfully saved icon:square for area '{}'",
                                    area.name()
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to save icon:square for area '{}': {}",
                                    area.name(),
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch URL {}: {}", url, e);
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to check areas without icon:square tag: {}", e);
        }
    }
}
