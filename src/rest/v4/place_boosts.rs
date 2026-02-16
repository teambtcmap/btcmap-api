use crate::db;
use crate::db::conf::schema::Conf;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use std::i64;

#[derive(Serialize)]
pub struct Quote {
    pub quote_30d_sat: i64,
    pub quote_90d_sat: i64,
    pub quote_365d_sat: i64,
}

#[get("/quote")]
pub async fn get_quote(conf: Data<Conf>) -> RestResult<Quote> {
    Ok(Json(Quote {
        quote_30d_sat: conf.paywall_boost_element_30d_price_sat,
        quote_90d_sat: conf.paywall_boost_element_90d_price_sat,
        quote_365d_sat: conf.paywall_boost_element_365d_price_sat,
    }))
}

#[derive(Deserialize)]
pub struct PostArgs {
    pub place_id: String,
    pub days: i64,
}

#[derive(Serialize)]
pub struct PostResponse {
    pub invoice_id: String,
    pub invoice: String,
}

#[post("")]
pub async fn post(
    req: HttpRequest,
    args: Json<PostArgs>,
    conf: Data<Conf>,
    pool: Data<Pool>,
) -> RestResult<PostResponse> {
    if args.place_id == "23143" {
        if let Some(ip) = req.connection_info().realip_remote_addr() {
            db::ban::queries::insert(ip.to_string(), "spam".to_string(), 3650, &pool)
                .await
                .map_err(|_| RestApiError::database())?;
            return Err(RestApiError::new(
                crate::rest::error::RestApiErrorCode::InvalidInput,
                "invalid request",
            ));
        }
    }

    let element = db::element::queries::select_by_id_or_osm_id(&args.place_id, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    let sats = match args.days {
        30 => conf.paywall_boost_element_30d_price_sat,
        90 => conf.paywall_boost_element_90d_price_sat,
        365 => conf.paywall_boost_element_365d_price_sat,
        _ => Err(RestApiError::new(
            crate::rest::error::RestApiErrorCode::InvalidInput,
            "invalid duration",
        ))?,
    };
    let invoice = service::invoice::create(
        "lnd",
        format!("element_boost:{}:{}", element.id, args.days),
        sats,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    Ok(Json(PostResponse {
        invoice_id: invoice.uuid,
        invoice: invoice.payment_request,
    }))
}
