use actix_web::{get, web::Query, HttpResponse, Responder};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::ApiError;

#[derive(Deserialize)]
pub struct GetArgs {
    theme: String,
    zoom: i32,
    x: i32,
    y: i32,
}

#[get("")]
async fn get(args: Query<GetArgs>) -> Result<impl Responder, ApiError> {
    let api_key = std::env::var("STADIA_API_KEY").unwrap();
    let url = format!(
        "https://tiles.stadiamaps.com/tiles/{}/{}/{}/{}@2x.png?api_key={}",
        args.theme, args.zoom, args.x, args.y, api_key
    );
    let response = reqwest::get(url).await.unwrap();

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("image/png")
        .body(response.bytes().await.unwrap()))
}
