use actix_cors::Cors;
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

/// CORS middleware for the API.
///
/// Allowed origins are read from the `BTCMAP_API_CORS_ORIGINS` env var:
/// - unset or `*` (the default): every origin is allowed
/// - comma-separated list of origins: only those are allowed
///
/// The middleware always allows every method and every header, and caches
/// preflight responses for 1 hour, which is enough for any other browser
/// client to use the API without CORS errors.
fn build_cors() -> Cors {
    let mut cors = Cors::default()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    match env::var("BTCMAP_API_CORS_ORIGINS") {
        Ok(value) if value.trim() == "*" => cors.allow_any_origin(),
        Ok(value) => {
            for origin in value.split(',') {
                let origin = origin.trim();
                if !origin.is_empty() {
                    cors = cors.allowed_origin(origin);
                }
            }
            cors
        }
        Err(_) => cors.allow_any_origin(),
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    init_env();

    let main_pool = db::main::pool()?;
    let image_pool = db::image::pool()?;
    let log_pool = db::log::pool()?;

    let conf = db::main::conf::queries::select(&main_pool).await?;

    // Trusted external base URL of this API. Used by the NIP-98 NostrAuth
    // extractor to reconstruct the URL the signed event must bind to.
    // Per-deployment infrastructure value, so it lives in env, not in Conf
    // (which is DB-backed and meant for runtime-tunable values shared across
    // deployments). Production must set this to the public URL.
    let api_base_url =
        env::var("BTCMAP_API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());

    check_areas_without_icon_square(&main_pool).await;
    backfill_og_image_metadata(&image_pool).await;

    service::matrix::init(&main_pool);

    HttpServer::new(move || {
        App::new()
            .wrap(Log)
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .wrap(from_fn(service::ban::check_if_banned))
            .wrap(build_cors())
            .app_data(Data::new(main_pool.clone()))
            .app_data(Data::new(image_pool.clone()))
            .app_data(Data::new(log_pool.clone()))
            .app_data(Data::new(conf.clone()))
            .app_data(Data::new(rest::nostr_auth::ApiBaseUrl(
                api_base_url.clone(),
            )))
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
                            .service(rest::v4::places::post_saved)
                            .service(rest::v4::places::delete_saved)
                            .service(rest::v4::places::get)
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
                            .service(rest::v4::place_issues::get)
                            .service(rest::v4::place_issues::get_by_id),
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
                            .service(rest::v4::areas::post_saved)
                            .service(rest::v4::areas::delete_saved)
                            .service(rest::v4::areas::get_by_id_top_editors)
                            .service(rest::v4::areas::get_by_id_image)
                            .service(rest::v4::areas::get_by_id)
                            .service(rest::v4::areas::get),
                    )
                    .service(scope("auth").service(rest::v4::nostr::auth_nostr))
                    .service(scope("dashboard").service(rest::v4::dashboard::get))
                    .service(scope("top-editors").service(rest::v4::top_editors::get))
                    .service(scope("communities").service(rest::v4::communities::get_top))
                    .service(scope("countries").service(rest::v4::countries::get_top))
                    .service(scope("activity").service(rest::v4::activity::get))
                    .service(
                        scope("users")
                            .service(rest::v4::users::me)
                            .service(rest::v4::users::post)
                            .service(rest::v4::users::change_password)
                            .service(rest::v4::users::update_username)
                            .service(rest::v4::users::get_nostr)
                            .service(rest::v4::users::put_nostr)
                            .service(rest::v4::users::delete_nostr)
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

async fn backfill_og_image_metadata(image_pool: &deadpool_sqlite::Pool) {
    use crate::db::image::og::queries;
    use image::ImageReader;
    use std::io::Cursor;

    let pending = match queries::select_all_with_zero_metadata(image_pool).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to load og images for metadata backfill: {}", e);
            return;
        }
    };

    if pending.is_empty() {
        tracing::info!("All cached og images already have metadata recorded");
        return;
    }

    tracing::warn!(
        count = pending.len(),
        "Backfilling metadata for cached og images"
    );

    let mut updated = 0usize;
    let mut failed = 0usize;
    for row in pending {
        let size = row.image_data.len() as i64;
        let dims = ImageReader::new(Cursor::new(&row.image_data))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.into_dimensions().ok());

        let Some((width, height)) = dims else {
            failed += 1;
            tracing::error!(
                element_id = row.element_id,
                "Failed to parse dimensions of cached og image"
            );
            continue;
        };

        match queries::update_metadata(
            row.element_id,
            width as i64,
            height as i64,
            size,
            image_pool,
        )
        .await
        {
            Ok(1) => updated += 1,
            Ok(_) => {
                failed += 1;
                tracing::warn!(
                    element_id = row.element_id,
                    "og image row vanished before metadata backfill"
                );
            }
            Err(e) => {
                failed += 1;
                tracing::error!(
                    element_id = row.element_id,
                    error = %e,
                    "Failed to backfill og image metadata"
                );
            }
        }
    }

    tracing::warn!(updated, failed, "og image metadata backfill finished");
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

#[cfg(test)]
mod test {
    use actix_web::http::header::HeaderValue;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::{test, App};
    use std::env;

    use super::build_cors;

    #[test]
    async fn cors_preflight_succeeds_with_any_origin() {
        // SAFETY: tests in the same module run on the same thread by default,
        // and we only ever set this once per test.
        unsafe {
            env::set_var("BTCMAP_API_CORS_ORIGINS", "*");
        }

        let app = test::init_service(App::new().wrap(build_cors())).await;
        let req = TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/rpc")
            .insert_header(("Origin", "https://dashboard.example.com"))
            .insert_header(("Access-Control-Request-Method", "POST"))
            .insert_header((
                "Access-Control-Request-Headers",
                "content-type,authorization",
            ))
            .to_request();
        let res = test::call_service(&app, req).await;

        assert_eq!(StatusCode::OK, res.status());
        let allow_origin = res
            .headers()
            .get(actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("missing Access-Control-Allow-Origin");
        // actix-cors echoes the request Origin back when allow_any_origin()
        // is set, instead of writing a literal "*". Both forms are valid CORS
        // responses for a non-credentialed request; the browser only checks
        // that the value is present and matches the Origin.
        assert_eq!(
            allow_origin,
            HeaderValue::from_static("https://dashboard.example.com")
        );
    }

    #[test]
    async fn cors_preflight_succeeds_for_allowed_origin() {
        // SAFETY: see the note in the other test.
        unsafe {
            env::set_var("BTCMAP_API_CORS_ORIGINS", "https://allowed.example.com");
        }

        let app = test::init_service(App::new().wrap(build_cors())).await;
        let req = TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/rpc")
            .insert_header(("Origin", "https://allowed.example.com"))
            .insert_header(("Access-Control-Request-Method", "POST"))
            .insert_header((
                "Access-Control-Request-Headers",
                "content-type,authorization",
            ))
            .to_request();
        let res = test::call_service(&app, req).await;

        assert_eq!(StatusCode::OK, res.status());
        let allow_origin = res
            .headers()
            .get(actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("missing Access-Control-Allow-Origin");
        assert_eq!(
            allow_origin,
            HeaderValue::from_static("https://allowed.example.com")
        );
    }

    #[test]
    async fn cors_preflight_rejects_disallowed_origin() {
        // SAFETY: see the note in the other test.
        unsafe {
            env::set_var("BTCMAP_API_CORS_ORIGINS", "https://allowed.example.com");
        }

        let app = test::init_service(App::new().wrap(build_cors())).await;
        let req = TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/rpc")
            .insert_header(("Origin", "https://attacker.example.com"))
            .insert_header(("Access-Control-Request-Method", "POST"))
            .insert_header(("Access-Control-Request-Headers", "content-type"))
            .to_request();
        let res = test::call_service(&app, req).await;

        // When the origin isn't in the allow list, the middleware short-circuits
        // the preflight with a 400 (actix-cors's documented behaviour for an
        // origin that doesn't match). The browser only reads the status code
        // and the CORS headers, and either way the preflight fails.
        assert_ne!(StatusCode::OK, res.status());
        assert!(res
            .headers()
            .get(actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
    }
}
