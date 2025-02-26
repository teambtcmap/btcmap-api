use crate::{element::Element, Result};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use deadpool_sqlite::Pool;
use include_dir::include_dir;
use include_dir::Dir;
use staticmap::{tools::IconBuilder, StaticMapBuilder};

static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

#[get("/og/element/{id}")]
pub async fn get_element(id: Path<String>, pool: Data<Pool>) -> Result<impl Responder> {
    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(element_og(&id, &pool).await?))
}

async fn element_og(id: &str, pool: &Pool) -> Result<Vec<u8>> {
    let Some(element) = Element::select_by_id_or_osm_id_async(id, pool).await? else {
        return Err("Element not found".into());
    };
    let res = actix_web::web::block(move || {
        let mut map = StaticMapBuilder::default()
            .width(600)
            .height(315)
            .zoom(17)
            .lat_center(element.lat())
            .lon_center(element.lon())
            .build()?;
        let icon_bytes = ICONS_DIR.get_file("marker.png").unwrap().contents();
        let marker = IconBuilder::new()
            .lat_coordinate(element.lat())
            .lon_coordinate(element.lon())
            .x_offset(20.)
            .y_offset(53.)
            .data(icon_bytes)?
            .build()?;
        map.add_tool(marker);
        map.encode_png()
    })
    .await??;
    Ok(res)
}
