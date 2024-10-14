extern crate core;
use actix_governor::{Governor, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use actix_web::middleware::{Compress, NormalizePath};
use actix_web::{App, HttpServer};
pub use error::Error;
use time::OffsetDateTime;
mod admin;
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
use std::sync::Arc;
use tracing::info;
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
mod firewall;
mod rpc;
mod sync;
use actix_web::dev::Service;
use actix_web::http::header::HeaderValue;
use actix_web::web::{scope, service, Data, QueryConfig};
use futures_util::future::FutureExt;

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

    let mut conn = db::open_connection()?;
    db::migrate(&mut conn)?;
    drop(conn);

    // All the worker threads are sharing a single connection pool
    let pool = Arc::new(db::pool()?);

    let rate_limit_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(30)
        .key_extractor(get_key_extractor())
        .finish()
        .unwrap();

    let tile_rate_limit_conf = GovernorConfigBuilder::default()
        .per_millisecond(500)
        .burst_size(1000)
        .key_extractor(get_key_extractor())
        .finish()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                let req_query_string = req.query_string().to_string();
                let req_method = req.method().as_str().to_string();
                let req_path = req.path().to_string();
                let req_version = format!("{:?}", req.version());
                let req_time = OffsetDateTime::now_utc();
                let req_ip = req
                    .connection_info()
                    .peer_addr()
                    .unwrap_or_default()
                    .to_string();
                let req_real_ip = req
                    .connection_info()
                    .realip_remote_addr()
                    .unwrap_or_default()
                    .to_string();
                srv.call(req).map(move |res| {
                    if let Ok(res) = res.as_ref() {
                        let res_status = res.status().as_u16();
                        let res_time_sec = (OffsetDateTime::now_utc() - req_time).as_seconds_f64();
                        if res_time_sec > 5.0 {
                            info!(
                                req_query_string,
                                req_method,
                                req_path,
                                req_version,
                                req_ip,
                                req_real_ip,
                                res_status,
                                res_time_sec,
                            );
                        }
                    }
                    res
                })
            })
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .app_data(Data::new(pool.clone()))
            .app_data(QueryConfig::default().error_handler(error::query_error_handler))
            .service(
                service("rpc").guard(actix_web::guard::Post()).finish(
                    jsonrpc_v2::Server::new()
                        .with_data(jsonrpc_v2::Data::new(pool.clone()))
                        .with_method("getelement", rpc::get_element::run)
                        .with_method("get_element", rpc::get_element::run)
                        .with_method("setelementtag", rpc::set_element_tag::run)
                        .with_method("set_element_tag", rpc::set_element_tag::run)
                        .with_method("removeelementtag", rpc::remove_element_tag::run)
                        .with_method("remove_element_tag", rpc::remove_element_tag::run)
                        .with_method("boostelement", rpc::boost_element::run)
                        .with_method("boost_element", rpc::boost_element::run)
                        .with_method("addelementcomment", rpc::add_element_comment::run)
                        .with_method("add_element_comment", rpc::add_element_comment::run)
                        .with_method("generateelementissues", rpc::generate_element_issues::run)
                        .with_method("generate_element_issues", rpc::generate_element_issues::run)
                        .with_method("addarea", rpc::add_area::run)
                        .with_method("add_area", rpc::add_area::run)
                        .with_method("getarea", rpc::get_area::run)
                        .with_method("get_area", rpc::get_area::run)
                        .with_method("setareatag", rpc::set_area_tag::run)
                        .with_method("set_area_tag", rpc::set_area_tag::run)
                        .with_method("removeareatag", rpc::remove_area_tag::run)
                        .with_method("remove_area_tag", rpc::remove_area_tag::run)
                        .with_method("gettrendingcountries", rpc::get_trending_countries::run)
                        .with_method("get_trending_countries", rpc::get_trending_countries::run)
                        .with_method(
                            "getmostcommentedcountries",
                            rpc::get_most_commented_countries::run,
                        )
                        .with_method(
                            "get_most_commented_countries",
                            rpc::get_most_commented_countries::run,
                        )
                        .with_method("gettrendingcommunities", rpc::get_trending_communities::run)
                        .with_method(
                            "get_trending_communities",
                            rpc::get_trending_communities::run,
                        )
                        .with_method("removearea", rpc::remove_area::run)
                        .with_method("remove_area", rpc::remove_area::run)
                        .with_method(
                            "generateareaselementsmapping",
                            rpc::generate_areas_elements_mapping::run,
                        )
                        .with_method(
                            "generate_areas_elements_mapping",
                            rpc::generate_areas_elements_mapping::run,
                        )
                        .with_method("generatereports", rpc::generate_reports::run)
                        .with_method("generate_reports", rpc::generate_reports::run)
                        .with_method("generateelementicons", rpc::generate_element_icons::run)
                        .with_method("generate_element_icons", rpc::generate_element_icons::run)
                        .with_method(
                            "generateelementcategories",
                            rpc::generate_element_categories::run,
                        )
                        .with_method(
                            "generate_element_categories",
                            rpc::generate_element_categories::run,
                        )
                        .with_method("syncelements", rpc::sync_elements::run)
                        .with_method("sync_elements", rpc::sync_elements::run)
                        .with_method("addadmin", rpc::add_admin::run)
                        .with_method("add_admin", rpc::add_admin::run)
                        .with_method("addadminaction", rpc::add_admin_action::run)
                        .with_method("add_admin_action", rpc::add_admin_action::run)
                        .with_method("removeadminaction", rpc::remove_admin_action::run)
                        .with_method("remove_admin_action", rpc::remove_admin_action::run)
                        .with_method("getuseractivity", rpc::get_user_activity::run)
                        .with_method("get_user_activity", rpc::get_user_activity::run)
                        .with_method("search", rpc::search::run)
                        .with_method("setareaicon", rpc::set_area_icon::run)
                        .with_method("set_area_icon", rpc::set_area_icon::run)
                        .with_method("getboostedelements", rpc::get_boosted_elements::run)
                        .with_method("get_boosted_elements", rpc::get_boosted_elements::run)
                        .finish()
                        .into_actix_web_service(),
                ),
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
                    .service(
                        scope("elements")
                            .service(element::admin::patch)
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
                            .service(user::admin::patch_tags)
                            .service(user::v2::get)
                            .service(user::v2::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(area::admin::patch)
                            .service(area::admin::delete)
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
                            .service(area::admin::patch)
                            .service(area::admin::delete)
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
                            .service(user::admin::patch_tags)
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
                scope("")
                    .wrap(Governor::new(&rate_limit_conf))
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
                            .service(user::admin::patch_tags)
                            .service(user::v2::get)
                            .service(user::v2::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(area::admin::patch)
                            .service(area::admin::delete)
                            .service(area::v2::get)
                            .service(area::v2::get_by_url_alias),
                    )
                    .service(
                        scope("reports")
                            .service(report::v2::get)
                            .service(report::v2::get_by_id),
                    ),
            )
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
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
            .map(|it| it.clone())
            .ok_or_else(|| {
                SimpleKeyExtractionError::new("Could not extract real IP address from request")
            })
    }
}
