use crate::db;
use crate::db::main::place_report::blocking_queries::InsertArgs;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;

const ORIGIN: &str = "user";

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceReportArgs")]
pub struct PostArgs {
    #[ts(type = "number")]
    pub place_id: i64,
    pub r#type: String,
    #[ts(type = "Record<string, unknown>")]
    pub extra_fields: Option<JsonObject>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceReportResponse")]
pub struct PostResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub origin: String,
}

#[post("")]
pub async fn post(
    auth: Auth,
    args: Json<PostArgs>,
    pool: Data<MainPool>,
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

    let extra_fields = args.extra_fields.clone().unwrap_or_default();

    let insert_args = InsertArgs {
        place_id: args.place_id,
        origin_id: origin.id,
        r#type: args.r#type.clone(),
        extra_fields,
        ticket_url: None,
        submitted_by: Some(user.id),
    };
    let report = db::main::place_report::queries::insert(insert_args, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    Ok(Json(PostResponse {
        id: report.id,
        origin: origin.name,
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
}
