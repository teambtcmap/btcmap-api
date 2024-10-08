use super::db;
use crate::{area, area_element, element, element_comment, error, feed, rpc, user};
use crate::{event, tile};
use crate::{report, Result};
use actix_governor::{Governor, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::{Service, ServiceRequest};
use actix_web::http::header::HeaderValue;
use actix_web::web::QueryConfig;
use actix_web::web::{scope, service};
use actix_web::{
    middleware::{Compress, NormalizePath},
    web::Data,
    App, HttpServer,
};
use futures_util::future::FutureExt;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::info;

pub async fn run() -> Result<()> {
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
                        .with_method("setelementtag", rpc::set_element_tag::run)
                        .with_method("removeelementtag", rpc::remove_element_tag::run)
                        .with_method("boostelement", rpc::boost_element::run)
                        .with_method("addelementcomment", rpc::add_element_comment::run)
                        .with_method("generateelementissues", rpc::generate_element_issues::run)
                        .with_method("addarea", rpc::add_area::run)
                        .with_method("getarea", rpc::get_area::run)
                        .with_method("setareatag", rpc::set_area_tag::run)
                        .with_method("removeareatag", rpc::remove_area_tag::run)
                        .with_method("gettrendingcountries", rpc::get_trending_countries::run)
                        .with_method(
                            "getmostcommentedcountries",
                            rpc::get_most_commented_countries::run,
                        )
                        .with_method("gettrendingcommunities", rpc::get_trending_communities::run)
                        .with_method("removearea", rpc::remove_area::run)
                        .with_method(
                            "generateareaselementsmapping",
                            rpc::generate_areas_elements_mapping::run,
                        )
                        .with_method("generatereports", rpc::generate_reports::run)
                        .with_method("generateelementicons", rpc::generate_element_icons::run)
                        .with_method(
                            "generateelementcategories",
                            rpc::generate_element_categories::run,
                        )
                        .with_method("syncelements", rpc::sync_elements::run)
                        .with_method("addadmin", rpc::add_admin::run)
                        .with_method("addallowedaction", rpc::add_allowed_action::run)
                        .with_method("removeallowedaction", rpc::remove_allowed_action::run)
                        .with_method("getuseractivity", rpc::get_user_activity::run)
                        .with_method("search", rpc::search::run)
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
