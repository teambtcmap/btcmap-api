use crate::db;
use crate::db::image::place::blocking_queries::InsertArgs as ImageInsertArgs;
use crate::db::image::place::schema::PlaceImage;
use crate::db::image::place::schema::PlaceImageMeta;
use crate::db::image::ImagePool;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpResponse;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

/// Image type used for photos uploaded directly by signed-in users (as opposed
/// to `report` evidence, which is tied to a place report).
const IMAGE_TYPE: &str = "user";

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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub created_by: Option<i64>,
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
            created_by: val.created_by,
        }
    }
}

impl From<PlaceImage> for ListItem {
    fn from(val: PlaceImage) -> Self {
        ListItem {
            id: val.id,
            place_id: val.place_id,
            r#type: val.r#type,
            width: val.width,
            height: val.height,
            size_bytes: val.size_bytes,
            created_at: val.created_at,
            created_by: val.created_by,
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

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceImageArgs")]
pub struct PostArgs {
    pub data_base64: String,
}

/// Add an image to a place. Requires authentication; the uploaded image is
/// stored with `type = "user"` and attributed to the signed-in user.
#[post("{id}/images")]
pub async fn post(
    auth: Auth,
    path: Path<String>,
    args: Json<PostArgs>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> Res<ListItem> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;

    let id = path.into_inner();
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    let place_id = resolve_place_id(id, &pool).await?;

    let data_base64 = args.into_inner().data_base64;
    let decoded = actix_web::web::block(move || service::image::decode_upload_base64(&data_base64))
        .await
        .map_err(|_| RestApiError::database())?
        .map_err(RestApiError::invalid_input)?;

    let size_bytes = decoded.bytes.len() as i64;
    let insert_args = ImageInsertArgs {
        place_id,
        r#type: IMAGE_TYPE.to_string(),
        image_data: decoded.bytes,
        width: decoded.width as i64,
        height: decoded.height as i64,
        size_bytes,
        created_by: Some(user.id),
    };
    let image = db::image::place::queries::insert(insert_args, &image_pool)
        .await
        .map_err(|_| RestApiError::database())?;

    Ok(Json(image.into()))
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
    use super::ImageInsertArgs;
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::header::ContentType;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use base64::prelude::*;

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
        let args = ImageInsertArgs {
            place_id,
            r#type: r#type.to_string(),
            image_data: bytes,
            width: 100,
            height: 50,
            size_bytes,
            created_by: None,
        };
        let image = db::image::place::queries::insert(args, image_pool).await?;
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

    async fn seed_user_with_token(pool: &crate::db::main::MainPool) -> Result<(i64, String)> {
        let user = db::main::user::queries::insert("tester", "", pool).await?;
        let secret = "test-secret".to_string();
        db::main::access_token::queries::insert(
            user.id,
            String::new(),
            secret.clone(),
            vec![Role::User],
            pool,
        )
        .await?;
        Ok((user.id, secret))
    }

    #[test]
    async fn post_requires_auth() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/places").service(super::post)),
        )
        .await;

        let payload =
            serde_json::json!({ "data_base64": BASE64_STANDARD.encode(encode_png(4, 2)) })
                .to_string();
        let req = TestRequest::post()
            .uri("/places/42/images")
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }

    #[test]
    async fn post_creates_user_image_with_submitter() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let place_id = insert_place(&main_pool, 1).await?;
        let (user_id, secret) = seed_user_with_token(&main_pool).await?;
        let bytes = encode_png(4, 2);

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool.clone()))
                .service(scope("/places").service(super::post)),
        )
        .await;

        let payload =
            serde_json::json!({ "data_base64": BASE64_STANDARD.encode(&bytes) }).to_string();
        let req = TestRequest::post()
            .uri(&format!("/places/{place_id}/images"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res: super::ListItem = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.place_id, place_id);
        assert_eq!(res.r#type, "user");
        assert_eq!(res.created_by, Some(user_id));
        assert_eq!(res.width, 4);
        assert_eq!(res.height, 2);

        let stored = db::image::place::queries::select_by_id(res.id, &image_pool).await?;
        assert_eq!(stored.image_data, bytes);
        assert_eq!(stored.r#type, "user");
        assert_eq!(stored.created_by, Some(user_id));

        Ok(())
    }

    #[test]
    async fn post_returns_not_found_for_unknown_place() -> Result<()> {
        let main_pool = pool();
        let (_, secret) = seed_user_with_token(&main_pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/places").service(super::post)),
        )
        .await;

        let payload =
            serde_json::json!({ "data_base64": BASE64_STANDARD.encode(encode_png(4, 2)) })
                .to_string();
        let req = TestRequest::post()
            .uri("/places/9999/images")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[test]
    async fn post_rejects_invalid_image() -> Result<()> {
        let main_pool = pool();
        let place_id = insert_place(&main_pool, 1).await?;
        let (_, secret) = seed_user_with_token(&main_pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/places").service(super::post)),
        )
        .await;

        let payload = serde_json::json!({
            "data_base64": BASE64_STANDARD.encode(b"not an image"),
        })
        .to_string();
        let req = TestRequest::post()
            .uri(&format!("/places/{place_id}/images"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}
