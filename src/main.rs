use actix_web::error::InternalError;
use actix_web::middleware::{from_fn, Compress, ErrorHandlers, NormalizePath};
use actix_web::{web, App, HttpServer, ResponseError};
use error::Error;
use rest::error::{RestApiError, RestApiErrorCode};
mod error;
#[cfg(test)]
mod test;
use std::env;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
mod db_utils;
mod feed;
mod log;
mod rpc;
mod sync;
use actix_web::web::{scope, Data};
use log::Log;
mod db;
mod og;
mod rest;
mod service;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() -> Result<()> {
    init_env();
    let pool = db_utils::pool()?;
    db_utils::migrate_async(&pool).await?;
    service::event::enforce_v2_compat(&pool).await?;
    service::report::enforce_v2_compat(&pool).await?;
    let conf = db::conf::queries_async::select(&pool).await?;
    HttpServer::new(move || {
        App::new()
            .wrap(Log)
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .wrap(from_fn(service::ban::check_if_banned))
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(conf.clone()))
            .service(og::element::get_element)
            .service(
                scope("rpc")
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
                            .service(rest::v4::places::get)
                            .service(rest::v4::places::get_boosted)
                            .service(rest::v4::places::get_by_id)
                            .service(rest::v4::places::get_by_id_comments),
                    )
                    .service(
                        scope("place-issues")
                            .service(rest::v4::element_issues::get)
                            .service(rest::v4::element_issues::get_by_id),
                    )
                    .service(scope("search").service(rest::v4::search::get)),
            )
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}

fn init_env() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(Layer::new().json())
        .init();
}
