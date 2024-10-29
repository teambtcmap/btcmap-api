use super::get_area::get_area;
use crate::Result;
use actix_web::{
    post,
    web::{Data, Json, Query},
    HttpRequest, HttpResponse, Responder,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Deserialize, Clone)]
struct RequestBody {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Value,
}

#[derive(Serialize)]
struct ResponseBody {}

#[post("/")]
pub async fn handle(
    req: HttpRequest,
    body: Query<RequestBody>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<ResponseBody>> {
    let cloned_body = body.clone();
    let res = pool
        .get()
        .await?
        .interact(move |_| match cloned_body.method.as_str() {
            "get_area" => ResponseBody {},
            _ => ResponseBody {},
        })
        .await?;
    Ok(Json(res))
}
