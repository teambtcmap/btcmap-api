use super::db;
use crate::auth::AuthService;
use crate::element::ElementRepo;
use crate::event::model::EventRepo;
use crate::report::model::ReportRepo;
use crate::user::UserRepo;
use crate::{area, element, error, user};
use crate::{event, tile};
use crate::{report, Result};
use actix_governor::{Governor, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::{Service, ServiceRequest};
use actix_web::http::header::HeaderValue;
use actix_web::web::scope;
use actix_web::web::QueryConfig;
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
        let auth_service = AuthService::new(&pool);

        let element_repo = ElementRepo::new(&pool);
        let event_repo = EventRepo::new(&pool);
        let report_repo = ReportRepo::new(&pool);
        let user_repo = UserRepo::new(&pool);

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
            .app_data(Data::new(auth_service))
            .app_data(Data::new(element_repo))
            .app_data(Data::new(event_repo))
            .app_data(Data::new(report_repo))
            .app_data(Data::new(user_repo))
            .app_data(QueryConfig::default().error_handler(error::query_error_handler))
            .service(
                scope("tiles")
                    .wrap(Governor::new(&tile_rate_limit_conf))
                    .service(tile::controller::get),
            )
            .service(
                scope("v2")
                    .wrap(Governor::new(&rate_limit_conf))
                    .service(
                        scope("elements")
                            .service(element::admin::patch)
                            .service(element::admin::post_tags)
                            .service(element::admin::patch_tags)
                            .service(element::v2::get)
                            .service(element::v2::get_by_osm_type_and_id),
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
                            .service(area::admin::post)
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
                        scope("events")
                            .service(event::v3::get)
                            .service(event::v3::get_by_id),
                    )
                    .service(
                        scope("areas")
                            .service(area::admin::post)
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
                    ),
            )
            .service(
                scope("")
                    .wrap(Governor::new(&rate_limit_conf))
                    .service(
                        scope("elements")
                            .service(element::admin::post_tags)
                            .service(element::admin::patch_tags)
                            .service(element::v2::get)
                            .service(element::v2::get_by_osm_type_and_id),
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
                            .service(area::admin::post)
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
