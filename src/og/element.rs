use crate::{db, db::image::ImagePool, db::main::MainPool, service::og::element_og, Result};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};

#[get("/og/element/{id}")]
pub async fn get_element(
    id: Path<String>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> Result<impl Responder> {
    let id = id.into_inner();
    let element = db::main::element::queries::select_by_id_or_osm_id(&id, &pool).await?;
    let current_version = element.overpass_data.version.unwrap_or(0);

    if let Some(cached) =
        db::image::og::queries::select_by_element_id(element.id, &image_pool).await?
    {
        if cached.version >= current_version {
            return Ok(HttpResponse::Ok()
                .content_type("image/jpeg")
                .body(cached.image_data));
        }
        db::image::og::queries::delete(element.id, &image_pool).await?;
    }

    let image_data = element_og(&id, &pool).await?;
    db::image::og::queries::insert(element.id, current_version, image_data.clone(), &image_pool)
        .await?;

    Ok(HttpResponse::Ok()
        .content_type("image/jpeg")
        .body(image_data))
}
