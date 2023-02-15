use crate::command;
use crate::controller;
use crate::Result;
use actix_web::web;
use actix_web::web::scope;
use actix_web::{
    middleware::{Compress, Logger, NormalizePath},
    web::Data,
    App, HttpServer,
};

pub async fn run() -> Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .wrap(NormalizePath::trim())
            .wrap(Compress::default())
            .app_data(Data::new(command::db::open_connection().unwrap()))
            .app_data(web::FormConfig::default().limit(262_144))
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
                    .service(controller::area_v2::post_json)
                    .service(controller::area_v2::get)
                    .service(controller::area_v2::get_by_id)
                    .service(controller::area_v2::patch_by_id)
                    .service(controller::area_v2::post_tags)
                    .service(controller::area_v2::delete_by_id),
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
                            .service(controller::area_v2::post_json)
                            .service(controller::area_v2::get)
                            .service(controller::area_v2::get_by_id)
                            .service(controller::area_v2::patch_by_id)
                            .service(controller::area_v2::post_tags)
                            .service(controller::area_v2::delete_by_id),
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
