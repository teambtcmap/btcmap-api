use super::db;
use crate::area::AreaRepo;
use crate::auth::AuthService;
use crate::element::ElementRepo;
use crate::event::model::EventRepo;
use crate::report::model::ReportRepo;
use crate::user::UserRepo;
use crate::{area, element, user};
use crate::{event, tile};
use crate::{report, Result};
use actix_web::dev::Service;
use actix_web::web;
use actix_web::web::scope;
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

    HttpServer::new(move || {
        let auth_service = AuthService::new(&pool);
        let area_repo = AreaRepo::new(&pool);
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
                        info!(
                            req_query_string,
                            req_method,
                            req_path,
                            req_version,
                            req_ip,
                            req_real_ip,
                            res_status,
                            res_time_sec = (OffsetDateTime::now_utc() - req_time).as_seconds_f64(),
                        );
                    }

                    res
                })
            })
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .app_data(Data::new(db::open_connection().unwrap()))
            .app_data(Data::new(auth_service))
            .app_data(Data::new(area_repo))
            .app_data(Data::new(element_repo))
            .app_data(Data::new(event_repo))
            .app_data(Data::new(report_repo))
            .app_data(Data::new(user_repo))
            .app_data(web::FormConfig::default().limit(262_144))
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
                    .service(user::controller_v2::get)
                    .service(user::controller_v2::get_by_id)
                    .service(user::controller_v2::patch_tags),
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
                    .service(report::controller_v2::get)
                    .service(report::controller_v2::get_by_id)
                    .service(report::controller_v2::patch_tags),
            )
            .service(scope("tiles").service(tile::controller::get))
            .service(
                scope("v2")
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
                            .service(user::controller_v2::get)
                            .service(user::controller_v2::get_by_id)
                            .service(user::controller_v2::patch_tags),
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
                            .service(report::controller_v2::get)
                            .service(report::controller_v2::get_by_id)
                            .service(report::controller_v2::patch_tags),
                    )
                    .service(scope("tiles").service(tile::controller::get)),
            )
            .service(scope("v3").service(scope("elements").service(element::v3::get)))
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}
