use crate::{service::og::element_og, Result};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use deadpool_sqlite::Pool;

#[get("/og/element/{id}")]
pub async fn get_element(id: Path<String>, pool: Data<Pool>) -> Result<impl Responder> {
    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(element_og(&id, &pool).await?))
}
