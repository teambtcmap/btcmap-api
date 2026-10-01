use crate::db;
use crate::db::image::place::blocking_queries::InsertArgs as ImageInsertArgs;
use crate::db::image::ImagePool;
use crate::db::main::place_report::blocking_queries::InsertArgs;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::service;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;

const ORIGIN: &str = "user";
/// Image type used for evidence attached to place reports.
const PHOTO_TYPE: &str = "report";
const MAX_PHOTOS: usize = 5;

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceReportArgs")]
pub struct PostArgs {
    #[ts(type = "number")]
    pub place_id: i64,
    pub r#type: String,
    #[ts(type = "Record<string, unknown>")]
    pub extra_fields: Option<JsonObject>,
    #[ts(optional)]
    pub photos: Option<Vec<PostPhotoArgs>>,
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceReportPhotoArgs")]
pub struct PostPhotoArgs {
    pub data_base64: String,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceReportResponse")]
pub struct PostResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub origin: String,
    #[ts(type = "Array<number>")]
    pub photo_ids: Vec<i64>,
}

struct DecodedPhoto {
    bytes: Vec<u8>,
    width: i64,
    height: i64,
}

/// Decode and validate the optional evidence photos before anything is written,
/// so a rejected upload cannot leave a half-created report behind.
///
/// Base64 decoding and (especially) full image decoding are CPU- and
/// allocation-heavy, so they run on the blocking pool instead of an Actix
/// worker thread.
async fn decode_photos(photos: Vec<PostPhotoArgs>) -> Result<Vec<DecodedPhoto>, RestApiError> {
    if photos.len() > MAX_PHOTOS {
        return Err(RestApiError::invalid_input(format!(
            "at most {MAX_PHOTOS} photos are allowed"
        )));
    }

    actix_web::web::block(move || {
        photos
            .iter()
            .map(|photo| {
                let decoded = service::image::decode_upload_base64(&photo.data_base64)
                    .map_err(RestApiError::invalid_input)?;
                Ok(DecodedPhoto {
                    bytes: decoded.bytes,
                    width: decoded.width as i64,
                    height: decoded.height as i64,
                })
            })
            .collect::<Result<Vec<_>, RestApiError>>()
    })
    .await
    .map_err(|_| RestApiError::database())?
}

#[post("")]
pub async fn post(
    auth: Auth,
    args: Json<PostArgs>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> RestResult<PostResponse> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;

    let origin = db::main::place_import_origin::queries::select_by_name(ORIGIN.to_string(), &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .ok_or_else(|| {
            RestApiError::new(
                crate::rest::error::RestApiErrorCode::Database,
                format!("import origin '{ORIGIN}' is not configured"),
            )
        })?;

    let args = args.into_inner();
    let extra_fields = args.extra_fields.unwrap_or_default();
    let photos = decode_photos(args.photos.unwrap_or_default()).await?;

    let insert_args = InsertArgs {
        place_id: args.place_id,
        origin_id: origin.id,
        r#type: args.r#type,
        extra_fields,
        ticket_url: None,
        submitted_by: Some(user.id),
    };
    let report = db::main::place_report::queries::insert(insert_args, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let mut photo_ids = Vec::with_capacity(photos.len());
    for photo in photos {
        let size_bytes = photo.bytes.len() as i64;
        let insert_args = ImageInsertArgs {
            place_id: args.place_id,
            r#type: PHOTO_TYPE.to_string(),
            image_data: photo.bytes,
            width: photo.width,
            height: photo.height,
            size_bytes,
            created_by: Some(user.id),
        };
        let image = db::image::place::queries::insert(insert_args, &image_pool)
            .await
            .map_err(|_| RestApiError::database())?;
        photo_ids.push(image.id);
    }

    Ok(Json(PostResponse {
        id: report.id,
        origin: origin.name,
        photo_ids,
    }))
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::header::ContentType;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use base64::prelude::*;
    use serde_json::Map;

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
        pool.get()
            .await?
            .interact(|conn| {
                conn.execute(
                    "INSERT INTO place_import_origin (name, gitea_sync_enabled, gitea_label_id) VALUES ('user', 0, NULL)",
                    [],
                )?;
                Ok::<_, rusqlite::Error>(())
            })
            .await??;
        Ok((user.id, secret))
    }

    #[test]
    async fn post_requires_auth() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"place_id":1,"type":"verification","extra_fields":{"comment":"hi"}}"#
                    .as_bytes(),
            )
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn post_creates_report() -> Result<()> {
        let pool = pool();
        let (user_id, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"place_id":42,"type":"verification","extra_fields":{"comment":"had lunch there"}}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(1, res.id);
        assert_eq!("user", res.origin);

        let origin =
            db::main::place_import_origin::queries::select_by_name("user".to_string(), &pool)
                .await?
                .unwrap();
        let stored = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        assert_eq!(stored.place_id, 42);
        assert_eq!(stored.origin_id, origin.id);
        assert_eq!(stored.r#type, "verification");
        assert_eq!(stored.submitted_by, Some(user_id));
        assert_eq!(
            stored.extra_fields.get("comment").and_then(|v| v.as_str()),
            Some("had lunch there"),
        );

        Ok(())
    }

    #[test]
    async fn post_uses_hardcoded_user_origin() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"place_id":7,"type":"missing_payment_method"}"#.as_bytes())
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!("user", res.origin);

        let stored = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        let origin_name =
            db::main::place_import_origin::queries::select_by_name("user".to_string(), &pool)
                .await?
                .unwrap();
        assert_eq!(stored.origin_id, origin_name.id);

        Ok(())
    }

    #[test]
    async fn post_creates_a_new_row_on_every_call() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"place_id":42,"type":"verification"}"#.as_bytes())
            .to_request();
        let first: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"place_id":42,"type":"verification"}"#.as_bytes())
            .to_request();
        let second: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_ne!(first.id, second.id);

        Ok(())
    }

    #[test]
    async fn post_stores_extra_fields() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let mut extra = Map::new();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"place_id":42,"type":"verification","extra_fields":{"comment":"hello"}}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;
        let stored = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        extra.insert("comment".into(), serde_json::Value::String("hello".into()));
        assert_eq!(extra, stored.extra_fields);

        Ok(())
    }

    #[test]
    async fn post_ticket_url_is_always_none() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"place_id":42,"type":"verification","ticket_url":"https://example.com/ticket/1"}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;
        let stored = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        assert!(stored.ticket_url.is_none());

        Ok(())
    }

    #[test]
    async fn post_records_submitter_id() -> Result<()> {
        let pool = pool();
        let (user_id, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"place_id":99,"type":"verification"}"#.as_bytes())
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;
        let stored = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        assert_eq!(Some(user_id), stored.submitted_by);

        Ok(())
    }

    #[test]
    async fn post_stores_photos_in_image_db() -> Result<()> {
        let pool = pool();
        let image_pool = crate::db::image::test::pool();
        let (user_id, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(image_pool.clone()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let bytes = encode_png(4, 2);
        let payload = serde_json::json!({
            "place_id": 42,
            "type": "verification",
            "photos": [{ "data_base64": BASE64_STANDARD.encode(&bytes) }],
        })
        .to_string();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.photo_ids.len(), 1);

        let stored = db::image::place::queries::select_by_id(res.photo_ids[0], &image_pool).await?;
        assert_eq!(stored.place_id, 42);
        assert_eq!(stored.r#type, "report");
        assert_eq!(stored.image_data, bytes);
        assert_eq!(stored.width, 4);
        assert_eq!(stored.height, 2);
        assert_eq!(stored.created_by, Some(user_id));

        Ok(())
    }

    #[test]
    async fn post_rejects_invalid_photo_without_creating_report() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let payload = serde_json::json!({
            "place_id": 42,
            "type": "verification",
            "photos": [{ "data_base64": BASE64_STANDARD.encode(b"not an image") }],
        })
        .to_string();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[test]
    async fn post_rejects_too_many_photos_without_creating_report() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let photo = serde_json::json!({ "data_base64": BASE64_STANDARD.encode(encode_png(1, 1)) });
        let photos: Vec<serde_json::Value> =
            (0..super::MAX_PHOTOS + 1).map(|_| photo.clone()).collect();
        let payload = serde_json::json!({
            "place_id": 42,
            "type": "verification",
            "photos": photos,
        })
        .to_string();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let reports = db::main::place_report::queries::select_open_and_not_deleted(&pool).await?;
        assert!(reports.is_empty());

        Ok(())
    }

    #[test]
    async fn post_rejects_oversized_photo() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                // Raised to match the server, since max-size photos exceed the
                // 2MB body limit Actix applies by default.
                .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024 * 1024))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let oversized = vec![0u8; crate::service::image::MAX_UPLOAD_BYTES + 1];
        let payload = serde_json::json!({
            "place_id": 42,
            "type": "verification",
            "photos": [{ "data_base64": BASE64_STANDARD.encode(&oversized) }],
        })
        .to_string();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let reports = db::main::place_report::queries::select_open_and_not_deleted(&pool).await?;
        assert!(reports.is_empty());

        Ok(())
    }

    #[test]
    async fn post_rejects_oversized_photo_dimensions() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/place-reports").service(super::post)),
        )
        .await;

        let bytes = encode_png(crate::service::image::MAX_UPLOAD_DIMENSION + 1, 1);
        let payload = serde_json::json!({
            "place_id": 42,
            "type": "verification",
            "photos": [{ "data_base64": BASE64_STANDARD.encode(&bytes) }],
        })
        .to_string();
        let req = TestRequest::post()
            .uri("/place-reports")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let reports = db::main::place_report::queries::select_open_and_not_deleted(&pool).await?;
        assert!(reports.is_empty());

        Ok(())
    }

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgb([255, 0, 0]));
        let mut out: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut out);
        img.write_with_encoder(encoder).unwrap();
        out
    }
}
