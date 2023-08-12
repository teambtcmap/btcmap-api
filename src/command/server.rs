use crate::command;
use crate::controller;
use crate::Result;
use actix_web::dev::Service;
use actix_web::web;
use actix_web::web::scope;
use actix_web::{
    middleware::{Compress, NormalizePath},
    web::Data,
    App, HttpServer,
};
use futures_util::future::FutureExt;
use time::OffsetDateTime;
use tracing::info;

pub async fn run() -> Result<()> {
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
            .app_data(Data::new(command::db::open_connection().unwrap()))
            .app_data(web::FormConfig::default().limit(262_144))
            .service(
                scope("elements")
                    .service(controller::element_v2::get)
                    .service(controller::element_v2::get_by_id)
                    .service(controller::element_v2::patch_tags)
                    .service(controller::element_v2::post_tags),
            )
            .service(
                scope("events")
                    .service(controller::event_v2::get)
                    .service(controller::event_v2::get_by_id)
                    .service(controller::event_v2::patch_tags),
            )
            .service(
                scope("users")
                    .service(controller::user_v2::get)
                    .service(controller::user_v2::get_by_id)
                    .service(controller::user_v2::patch_tags),
            )
            .service(
                scope("areas")
                    .service(controller::area_v2::post_json)
                    .service(controller::area_v2::get)
                    .service(controller::area_v2::get_by_id)
                    .service(controller::area_v2::patch_tags)
                    .service(controller::area_v2::patch_by_id)
                    .service(controller::area_v2::post_tags)
                    .service(controller::area_v2::delete_by_id),
            )
            .service(
                scope("reports")
                    .service(controller::report_v2::get)
                    .service(controller::report_v2::get_by_id)
                    .service(controller::report_v2::patch_tags),
            )
            .service(scope("tiles").service(controller::tile::get))
            .service(
                scope("v2")
                    .service(
                        scope("elements")
                            .service(controller::element_v2::get)
                            .service(controller::element_v2::get_by_id)
                            .service(controller::element_v2::patch_tags)
                            .service(controller::element_v2::post_tags),
                    )
                    .service(
                        scope("events")
                            .service(controller::event_v2::get)
                            .service(controller::event_v2::get_by_id)
                            .service(controller::event_v2::patch_tags),
                    )
                    .service(
                        scope("users")
                            .service(controller::user_v2::get)
                            .service(controller::user_v2::get_by_id)
                            .service(controller::user_v2::patch_tags),
                    )
                    .service(
                        scope("areas")
                            .service(controller::area_v2::post_json)
                            .service(controller::area_v2::get)
                            .service(controller::area_v2::get_by_id)
                            .service(controller::area_v2::patch_tags)
                            .service(controller::area_v2::patch_by_id)
                            .service(controller::area_v2::post_tags)
                            .service(controller::area_v2::delete_by_id),
                    )
                    .service(
                        scope("reports")
                            .service(controller::report_v2::get)
                            .service(controller::report_v2::get_by_id)
                            .service(controller::report_v2::patch_tags),
                    )
                    .service(scope("tiles").service(controller::tile::get)),
            )
            .service(
                scope("v3").service(
                    scope("elements")
                        .service(controller::element_v3::get)
                        .service(controller::element_v3::get_by_id)
                        .service(controller::element_v3::patch_tags)
                        .service(controller::element_v3::post_tags),
                ),
            )
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}
