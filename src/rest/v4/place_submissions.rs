use crate::db;
use crate::db::main::place_submission::blocking_queries::InsertArgs;
use crate::db::main::place_submission::schema::PlaceSubmission;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestApiErrorCode;
use crate::rest::error::RestResult;
use actix_web::get;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

const ORIGIN: &str = "user";

#[derive(Deserialize)]
pub struct Args {
    pub source: Option<String>,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export, rename = "PlaceSubmission")]
pub struct Item {
    #[ts(type = "number")]
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    #[ts(type = "Record<string, unknown>")]
    pub extra_fields: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    pub revoked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub submitted_by: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(optional, type = "string")]
    pub closed_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(optional, type = "string")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<PlaceSubmission> for Item {
    fn from(val: PlaceSubmission) -> Self {
        Item {
            id: val.id,
            origin: val.origin,
            external_id: val.external_id,
            lat: val.lat,
            lon: val.lon,
            category: val.category,
            name: val.name,
            extra_fields: val.extra_fields,
            ticket_url: val.ticket_url.map(humanize_ticket_url),
            revoked: val.revoked,
            submitted_by: val.submitted_by,
            created_at: val.created_at,
            updated_at: val.updated_at,
            closed_at: val.closed_at,
            deleted_at: val.deleted_at,
        }
    }
}

fn humanize_ticket_url(url: String) -> String {
    url.replacen("/api/v1/repos", "", 1)
}

impl From<PlaceSubmission> for Json<Item> {
    fn from(val: PlaceSubmission) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(args: Query<Args>, pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let items = match args.source.as_deref() {
        Some(origin) => {
            db::main::place_submission::queries::select_open_and_not_revoked_by_origin(
                origin.to_string(),
                &pool,
            )
            .await
        }
        None => db::main::place_submission::queries::select_open_and_not_revoked(&pool).await,
    }
    .map_err(|_| RestApiError::database())?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}
#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceSubmissionArgs")]
pub struct PostArgs {
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    #[ts(type = "Record<string, unknown>")]
    pub extra_fields: Option<JsonObject>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostPlaceSubmissionResponse")]
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

    if !(-90.0..=90.0).contains(&args.lat) {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Latitude must be between -90 and 90",
        ));
    }

    if !(-180.0..=180.0).contains(&args.lon) {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Longitude must be between -180 and 180",
        ));
    }

    if args.category.trim().is_empty() {
        return Err(RestApiError::invalid_input("category cannot be empty"));
    }

    if args.name.trim().is_empty() {
        return Err(RestApiError::invalid_input("name cannot be empty"));
    }

    let extra_fields = args.extra_fields.clone().unwrap_or_default();

    let insert_args = InsertArgs {
        origin: ORIGIN.to_string(),
        external_id: Uuid::new_v4().to_string(),
        lat: args.lat,
        lon: args.lon,
        category: args.category.clone(),
        name: args.name.clone(),
        extra_fields,
        submitted_by: Some(user.id),
    };
    let submission = db::main::place_submission::queries::insert(insert_args, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    Ok(Json(PostResponse {
        id: submission.id,
        origin: submission.origin,
    }))
}

#[cfg(test)]
mod test {
    use crate::db::main::place_submission::blocking_queries::InsertArgs;
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

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_returns_open_and_not_revoked() -> Result<()> {
        let pool = pool();

        let open_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "1".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Open Place".to_string(),
            extra_fields: Map::new(),
            submitted_by: None,
        };
        let open = db::main::place_submission::queries::insert(open_args, &pool).await?;

        let revoked_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "2".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Revoked Place".to_string(),
            extra_fields: Map::new(),
            submitted_by: None,
        };
        let revoked = db::main::place_submission::queries::insert(revoked_args, &pool).await?;
        db::main::place_submission::queries::set_revoked(revoked.id, true, &pool).await?;

        let closed_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "3".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Closed Place".to_string(),
            extra_fields: Map::new(),
            submitted_by: None,
        };
        let closed = db::main::place_submission::queries::insert(closed_args, &pool).await?;
        db::main::place_submission::queries::set_closed_at(
            closed.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(1, res.len());
        assert_eq!(open.id, res[0]["id"].as_i64().unwrap());
        assert_eq!(false, res[0]["revoked"].as_bool().unwrap());
        assert!(res[0]["closed_at"].is_null());

        Ok(())
    }

    #[test]
    async fn get_filters_by_source() -> Result<()> {
        let pool = pool();

        for (origin, external_id) in [("square", "1"), ("coinos", "1"), ("coinos", "2")] {
            let args = InsertArgs {
                origin: origin.to_string(),
                external_id: external_id.to_string(),
                lat: 1.23,
                lon: 4.56,
                category: "cafe".to_string(),
                name: format!("{origin} place"),
                extra_fields: Map::new(),
                submitted_by: None,
            };
            db::main::place_submission::queries::insert(args, &pool).await?;
        }

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?source=coinos").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        assert!(res.iter().all(|r| r["origin"] == "coinos"));

        let req = TestRequest::get().uri("/?source=square").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("square", res[0]["origin"]);

        let req = TestRequest::get()
            .uri("/?source=does_not_exist")
            .to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());

        Ok(())
    }

    #[test]
    async fn get_rewrites_ticket_url_to_web_link() -> Result<()> {
        let pool = pool();
        let args = InsertArgs {
            origin: "square".to_string(),
            external_id: "url-rewrite".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "URL rewrite probe".to_string(),
            extra_fields: Map::new(),
            submitted_by: None,
        };
        let submission = db::main::place_submission::queries::insert(args, &pool).await?;
        let api_url = "https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues/42";
        db::main::place_submission::queries::set_ticket_url(
            submission.id,
            api_url.to_string(),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(
            "https://gitea.btcmap.org/teambtcmap/btcmap-data/issues/42",
            res[0]["ticket_url"].as_str().unwrap(),
        );
        Ok(())
    }

    #[test]
    async fn humanize_ticket_url_strips_api_prefix() {
        assert_eq!(
            "https://gitea.btcmap.org/teambtcmap/btcmap-data/issues/1",
            super::humanize_ticket_url(
                "https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues/1".to_string(),
            ),
        );
    }

    #[test]
    async fn humanize_ticket_url_leaves_non_gitea_urls_alone() {
        let url = "https://example.com/issue/123".to_string();
        assert_eq!(url.clone(), super::humanize_ticket_url(url));
    }

    #[test]
    async fn humanize_ticket_url_strips_only_first_occurrence() {
        assert_eq!(
            "https://gitea.btcmap.org/foo/api/v1/repos/bar",
            super::humanize_ticket_url(
                "https://gitea.btcmap.org/api/v1/repos/foo/api/v1/repos/bar".to_string(),
            ),
        );
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
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":1.0,"lon":2.0,"category":"cafe","name":"Cafe"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn post_creates_submission() -> Result<()> {
        let pool = pool();
        let (user_id, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"lat":18.2649,"lon":98.5013,"category":"cafe","name":"Satoshi Cafe","extra_fields":{"website":"https://example.com"}}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.id, 1);
        assert_eq!(res.origin, "user");

        let stored = db::main::place_submission::queries::select_by_id(res.id, &pool).await?;
        assert_eq!(stored.origin, "user");
        assert_eq!(stored.submitted_by, Some(user_id));
        assert_eq!(stored.lat, 18.2649);
        assert_eq!(stored.lon, 98.5013);
        assert_eq!(stored.category, "cafe");
        assert_eq!(stored.name, "Satoshi Cafe");
        assert_eq!(
            stored.extra_fields.get("website").and_then(|v| v.as_str()),
            Some("https://example.com"),
        );

        Ok(())
    }

    #[test]
    async fn post_assigns_a_uuid_external_id_for_db_compat() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"lat":18.2649,"lon":98.5013,"category":"cafe","name":"Satoshi Cafe"}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        let stored = db::main::place_submission::queries::select_by_id(res.id, &pool).await?;
        assert!(
            uuid::Uuid::parse_str(&stored.external_id).is_ok(),
            "expected stored external_id to be a UUID, got {:?}",
            stored.external_id,
        );

        Ok(())
    }

    #[test]
    async fn post_generates_unique_external_ids_across_calls() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":1.0,"lon":2.0,"category":"cafe","name":"First"}"#.as_bytes())
            .to_request();
        let first: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":3.0,"lon":4.0,"category":"cafe","name":"Second"}"#.as_bytes())
            .to_request();
        let second: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        assert_ne!(first.id, second.id);

        let first_stored =
            db::main::place_submission::queries::select_by_id(first.id, &pool).await?;
        let second_stored =
            db::main::place_submission::queries::select_by_id(second.id, &pool).await?;
        assert_ne!(first_stored.external_id, second_stored.external_id);

        Ok(())
    }

    #[test]
    async fn post_ignores_client_supplied_external_id() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(
                r#"{"external_id":"client-supplied","lat":1.0,"lon":2.0,"category":"cafe","name":"Cafe"}"#
                    .as_bytes(),
            )
            .to_request();
        let res: super::PostResponse = test::call_and_read_body_json(&app, req).await;

        let stored = db::main::place_submission::queries::select_by_id(res.id, &pool).await?;
        assert_ne!(stored.external_id, "client-supplied");

        Ok(())
    }

    #[test]
    async fn post_rejects_out_of_range_lat() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":120.0,"lon":0.0,"category":"cafe","name":"Cafe"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[test]
    async fn post_rejects_out_of_range_lon() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":0.0,"lon":200.0,"category":"cafe","name":"Cafe"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[test]
    async fn post_rejects_empty_category() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":0.0,"lon":0.0,"category":"   ","name":"Cafe"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[test]
    async fn post_rejects_empty_name() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/place-submissions").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/place-submissions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":0.0,"lon":0.0,"category":"cafe","name":"   "}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}
