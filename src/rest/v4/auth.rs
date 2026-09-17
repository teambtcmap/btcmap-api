use crate::db;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::{RestApiError, RestResult};
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SignoutResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub label: Option<String>,
    #[ts(type = "string", optional)]
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub revoked_at: Option<OffsetDateTime>,
}

/// `POST /v4/auth/signout`
///
/// Revoke the Bearer token that authenticates this request. Only the
/// presented token is affected; other tokens for the same user stay valid.
///
/// The [`Auth`] extractor only resolves tokens whose `deleted_at` is unset,
/// so a second call with the same token is rejected with `401` rather than
/// being silently idempotent.
#[post("/signout")]
pub async fn signout(auth: Auth, pool: Data<MainPool>) -> RestResult<SignoutResponse> {
    let token = auth.token.ok_or_else(RestApiError::unauthorized)?;
    let token = db::main::access_token::queries::set_deleted_at(
        token.id,
        Some(OffsetDateTime::now_utc()),
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    Ok(Json(SignoutResponse {
        id: token.id,
        label: token.name,
        revoked_at: token.deleted_at,
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db;
    use crate::db::main::test::pool;
    use crate::Result;
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};

    fn build_app(
        pool_data: Data<MainPool>,
    ) -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .app_data(pool_data)
            .service(scope("/auth").service(signout))
    }

    #[actix_web::test]
    async fn missing_token_returns_401() -> Result<()> {
        let app = test::init_service(build_app(Data::new(pool()))).await;
        let req = TestRequest::post().uri("/auth/signout").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[actix_web::test]
    async fn valid_token_is_revoked() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("alice", "", &pool).await?;
        let token = db::main::access_token::queries::insert(
            user.id,
            "laptop".into(),
            "secret".into(),
            vec![],
            &pool,
        )
        .await?;

        let app = test::init_service(build_app(Data::new(pool.clone()))).await;
        let req = TestRequest::post()
            .uri("/auth/signout")
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .to_request();
        let res: SignoutResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.id, token.id);
        assert_eq!(res.label.as_deref(), Some("laptop"));
        assert!(res.revoked_at.is_some());

        let refreshed = db::main::access_token::queries::select_by_id(token.id, &pool).await?;
        assert!(refreshed.deleted_at.is_some());
        Ok(())
    }

    #[actix_web::test]
    async fn revoked_token_is_rejected() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("alice", "", &pool).await?;
        let token = db::main::access_token::queries::insert(
            user.id,
            "laptop".into(),
            "secret".into(),
            vec![],
            &pool,
        )
        .await?;
        db::main::access_token::queries::set_deleted_at(
            token.id,
            Some(OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;

        let app = test::init_service(build_app(Data::new(pool))).await;
        let req = TestRequest::post()
            .uri("/auth/signout")
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[actix_web::test]
    async fn other_tokens_are_untouched() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("alice", "", &pool).await?;
        db::main::access_token::queries::insert(
            user.id,
            "laptop".into(),
            "secret".into(),
            vec![],
            &pool,
        )
        .await?;
        let other = db::main::access_token::queries::insert(
            user.id,
            "phone".into(),
            "other".into(),
            vec![],
            &pool,
        )
        .await?;

        let app = test::init_service(build_app(Data::new(pool.clone()))).await;
        let req = TestRequest::post()
            .uri("/auth/signout")
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .to_request();
        test::call_service(&app, req).await;

        let refreshed = db::main::access_token::queries::select_by_id(other.id, &pool).await?;
        assert!(refreshed.deleted_at.is_none());
        Ok(())
    }
}
