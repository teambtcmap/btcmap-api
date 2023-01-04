use crate::command;
use crate::controller;
use crate::Result;
use actix_web::web::scope;
use actix_web::{
    middleware::{Compress, Logger, NormalizePath},
    web::Data,
    App, HttpServer,
};

pub async fn run() -> Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .app_data(Data::new(command::db::open_connection().unwrap()))
            .service(
                scope("elements")
                    .service(controller::element_v2::get)
                    .service(controller::element_v2::get_by_id)
                    .service(controller::element_v2::post_tags),
            )
            .service(
                scope("events")
                    .service(controller::event_v2::get)
                    .service(controller::event_v2::get_by_id),
            )
            .service(
                scope("users")
                    .service(controller::user_v2::get)
                    .service(controller::user_v2::get_by_id)
                    .service(controller::user_v2::post_tags),
            )
            .service(
                scope("areas")
                    .service(controller::area_v2::post)
                    .service(controller::area_v2::get)
                    .service(controller::area_v2::get_by_id)
                    .service(controller::area_v2::post_tags),
            )
            .service(
                scope("reports")
                    .service(controller::report_v2::get)
                    .service(controller::report_v2::get_by_id)
                    .service(controller::report_v2::post_tags),
            )
            .service(
                scope("v2")
                    .service(
                        scope("elements")
                            .service(controller::element_v2::get)
                            .service(controller::element_v2::get_by_id)
                            .service(controller::element_v2::post_tags),
                    )
                    .service(
                        scope("events")
                            .service(controller::event_v2::get)
                            .service(controller::event_v2::get_by_id),
                    )
                    .service(
                        scope("users")
                            .service(controller::user_v2::get)
                            .service(controller::user_v2::get_by_id)
                            .service(controller::user_v2::post_tags),
                    )
                    .service(
                        scope("areas")
                            .service(controller::area_v2::post)
                            .service(controller::area_v2::get)
                            .service(controller::area_v2::get_by_id)
                            .service(controller::area_v2::post_tags),
                    )
                    .service(
                        scope("reports")
                            .service(controller::report_v2::get)
                            .service(controller::report_v2::get_by_id)
                            .service(controller::report_v2::post_tags),
                    ),
            )
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await?;

    Ok(())
}
