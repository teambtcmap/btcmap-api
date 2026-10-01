use crate::db;
use crate::db::image::place::schema::PlaceImageMeta;
use crate::db::image::ImagePool;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpResponse;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetListArgs {
    r#type: Option<String>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "PlaceImage")]
pub struct ListItem {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub place_id: i64,
    pub r#type: String,
    #[ts(type = "number")]
    pub width: i64,
    #[ts(type = "number")]
    pub height: i64,
    #[ts(type = "number")]
    pub size_bytes: i64,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
}

impl From<PlaceImageMeta> for ListItem {
    fn from(val: PlaceImageMeta) -> Self {
        ListItem {
            id: val.id,
            place_id: val.place_id,
            r#type: val.r#type,
            width: val.width,
            height: val.height,
            size_bytes: val.size_bytes,
            created_at: val.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct GetImageArgs {
    pub w: Option<u32>,
    pub h: Option<u32>,
}

/// List the images attached to a place. Filter with `type`, e.g. `report`.
#[get("{id}/images")]
pub async fn get_by_place_id(
    id: Path<String>,
    args: Query<GetListArgs>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> Res<Vec<ListItem>> {
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    let place_id = resolve_place_id(id.into_inner(), &pool).await?;
    let images = match args.r#type.as_deref() {
        Some(r#type) => {
            db::image::place::queries::select_by_place_id_and_type(place_id, r#type, &image_pool)
                .await
        }
        None => db::image::place::queries::select_by_place_id(place_id, &image_pool).await,
    }
    .map_err(|_| RestApiError::database())?;
    Ok(Json(images.into_iter().map(Into::into).collect()))
}

/// Serve a single place image. Public; optionally resized with `w`/`h`.
#[get("{id}/images/{image_id}")]
pub async fn get_by_place_id_and_image_id(
    path: Path<(String, i64)>,
    args: Query<GetImageArgs>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> Result<HttpResponse, RestApiError> {
    let (id, image_id) = path.into_inner();
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    if let Some(0) = args.w {
        return Err(RestApiError::invalid_input("w must be greater than 0"));
    }
    if let Some(0) = args.h {
        return Err(RestApiError::invalid_input("h must be greater than 0"));
    }
    let place_id = resolve_place_id(id, &pool).await?;
    let image = db::image::place::queries::select_by_id(image_id, &image_pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    if image.place_id != place_id {
        return Err(RestApiError::not_found());
    }
    let (bytes, content_type) = service::image::render(image.image_data, args.w, args.h)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(HttpResponse::Ok().content_type(content_type).body(bytes))
}

async fn resolve_place_id(id: String, pool: &MainPool) -> Result<i64, RestApiError> {
    db::main::element::queries::select_by_id_or_osm_id(id, pool)
        .await
        .map(|it| it.id)
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgb([255, 0, 0]));
        let mut out: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut out);
        img.write_with_encoder(encoder).unwrap();
        out
    }

    async fn insert_place(pool: &crate::db::main::MainPool, osm_id: i64) -> Result<i64> {
        let element =
            db::main::element::queries::insert(OverpassElement::mock(osm_id), pool).await?;
        Ok(element.id)
    }

    async fn insert_image(
        image_pool: &crate::db::image::ImagePool,
        place_id: i64,
        r#type: &str,
        bytes: Vec<u8>,
    ) -> Result<i64> {
        let size_bytes = bytes.len() as i64;
        let image = db::image::place::queries::insert(
            place_id, r#type, bytes, 100, 50, size_bytes, image_pool,
        )
        .await?;
        Ok(image.id)
    }

    #[test]
    async fn list_returns_place_images_and_filters_by_type() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;
        insert_image(&image_pool, place_id, "report", vec![1, 2, 3]).await?;
        insert_image(&image_pool, place_id, "report", vec![4, 5, 6]).await?;
        insert_image(&image_pool, place_id, "cover", vec![7, 8, 9]).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/places").service(super::get_by_place_id)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/places/{place_id}/images"))
            .to_request();
        let res: Vec<super::ListItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 3);

        let req = TestRequest::get()
            .uri(&format!("/places/{place_id}/images?type=report"))
            .to_request();
        let res: Vec<super::ListItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);
        assert!(res.iter().all(|it| it.r#type == "report"));

        Ok(())
    }

    #[test]
    async fn serves_image_bytes() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;
        let bytes = encode_png(4, 2);
        let image_id = insert_image(&image_pool, place_id, "report", bytes.clone()).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/places").service(super::get_by_place_id_and_image_id)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/places/{place_id}/images/{image_id}"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/png"),
        );
        assert_eq!(test::read_body(res).await.to_vec(), bytes);

        Ok(())
    }

    #[test]
    async fn image_not_found_for_unknown_id() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/places").service(super::get_by_place_id_and_image_id)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/places/{place_id}/images/9999"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);

        Ok(())
    }

    #[test]
    async fn image_not_found_when_place_does_not_match() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;
        let other_id = insert_place(&main_pool, 2).await?;
        let image_id = insert_image(&image_pool, place_id, "report", encode_png(4, 2)).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/places").service(super::get_by_place_id_and_image_id)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/places/{other_id}/images/{image_id}"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);

        Ok(())
    }

    #[test]
    async fn image_rejects_zero_dimension() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;
        let image_id = insert_image(&image_pool, place_id, "report", encode_png(4, 2)).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/places").service(super::get_by_place_id_and_image_id)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/places/{place_id}/images/{image_id}?w=0"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);

        Ok(())
    }
}
