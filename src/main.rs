use actix_cors::Cors;
use actix_web::error::InternalError;
use actix_web::middleware::{from_fn, Compress, ErrorHandlers, NormalizePath};
use actix_web::{web, App, HttpServer, ResponseError};
use error::Error;
use rest::error::{RestApiError, RestApiErrorCode};
mod error;
use std::env;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod feed;
mod rpc;
use crate::db::main::conf::schema::Conf;
use crate::service::log::Log;
use actix_web::web::{scope, Data};
mod db;
mod og;
mod rest;
mod service;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// CORS middleware for the API.
///
/// Allowed origins are read from the `conf.cors_origins` DB column (a
/// comma-separated list):
/// - empty (the default): every origin is allowed
/// - one or more entries: only those origins are allowed
///
/// The middleware always allows every method and every header, and caches
/// preflight responses for 1 hour, which is enough for any other browser
/// client to use the API without CORS errors.
fn build_cors(conf: &Conf) -> Cors {
    let mut cors = Cors::default()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    if conf.cors_origins.is_empty() {
        cors = cors.allow_any_origin();
    } else {
        for origin in &conf.cors_origins {
            cors = cors.allowed_origin(origin);
        }
    }

    cors
}

#[actix_web::main]
async fn main() -> Result<()> {
    init_env();

    let main_pool = db::main::pool()?;
    let image_pool = db::image::pool()?;
    let log_pool = db::log::pool()?;

    let conf = db::main::conf::queries::select(&main_pool).await?;

    if conf.electrum_url.trim().is_empty()
        && (!conf.xpub_spending.trim().is_empty()
            || !conf.xpub_donations.trim().is_empty()
            || !conf.xpub_treasury.trim().is_empty())
    {
        tracing::warn!(
            "electrum_url is not configured in the conf table but at least one xpub is set. \
             The get_wallets RPC will return an error until electrum_url is configured."
        );
    }

    // Trusted external base URL of this API. Used by the NIP-98 NostrAuth
    // extractor to reconstruct the URL the signed event must bind to.
    // Per-deployment infrastructure value, so it lives in env, not in Conf
    // (which is DB-backed and meant for runtime-tunable values shared across
    // deployments). Production must set this to the public URL.
    let api_base_url =
        env::var("BTCMAP_API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());

    service::matrix::init(&main_pool);
    // Cancellation token shared with every long-lived background task so we
    // can break out of their loops before the actix runtime drops on SIGTERM.
    // See the note in `service::wallet_cache::init` for why this matters.
    let shutdown = CancellationToken::new();
    service::wallet_cache::init(&main_pool, shutdown.clone());

    HttpServer::new(move || {
        App::new()
            .wrap(Log)
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .wrap(from_fn(service::ban::check_if_banned))
            .wrap(build_cors(&conf))
            .app_data(Data::new(main_pool.clone()))
            .app_data(Data::new(image_pool.clone()))
            .app_data(Data::new(log_pool.clone()))
            .app_data(Data::new(conf.clone()))
            .app_data(web::PayloadConfig::new(64 * 1024 * 1024))
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

    // Signal background tasks to exit so they don't keep the actix runtime's
    // blocking pool alive while the runtime is being dropped.
    shutdown.cancel();

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

#[cfg(test)]
mod test {
    use super::build_cors;
    use crate::db::main::conf::schema::Conf;
    use actix_web::http::header::HeaderValue;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::{test, App};

    fn conf_with(origins: &[&str]) -> Conf {
        Conf {
            cors_origins: origins.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    async fn cors_preflight_succeeds_with_any_origin() {
        // Empty `cors_origins` is the default and means "allow any origin".
        let conf = conf_with(&[]);

        let app = test::init_service(App::new().wrap(build_cors(&conf))).await;
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
        let conf = conf_with(&["https://allowed.example.com"]);

        let app = test::init_service(App::new().wrap(build_cors(&conf))).await;
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
        let conf = conf_with(&["https://allowed.example.com"]);

        let app = test::init_service(App::new().wrap(build_cors(&conf))).await;
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
