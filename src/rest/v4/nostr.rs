use crate::db;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::service::nip98;
use actix_web::http::header;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

/// POST /v4/nostr/token
///
/// Obtain a BTC Map API token using NIP-98 Nostr auth.
/// The client sends `Authorization: Nostr <base64-encoded kind 27235 event>`.
/// The server verifies the event, looks up the user by pubkey, and returns a Bearer token.
#[post("/token")]
pub async fn create_token(
    req: HttpRequest,
    pool: Data<MainPool>,
) -> Result<Json<TokenResponse>, RestApiError> {
    let nostr_payload = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(nip98::extract_nostr_auth)
        .ok_or_else(|| {
            RestApiError::unauthorized_with_message(
                "Missing or invalid Authorization: Nostr header",
            )
        })?;

    let url = format!(
        "{}://{}{}",
        req.connection_info().scheme(),
        req.connection_info().host(),
        req.uri(),
    );

    let verified = nip98::verify(nostr_payload, &url, "POST")
        .map_err(|e| RestApiError::invalid_input(format!("NIP-98 verification failed: {e}")))?;

    let user = db::main::user::queries::select_by_npub(&verified.pubkey, &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .ok_or_else(|| {
            RestApiError::invalid_input("No BTC Map account linked to this Nostr pubkey")
        })?;

    let token = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(user.id, String::new(), token.clone(), vec![], &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    Ok(Json(TokenResponse { token }))
}

#[cfg(test)]
mod test {
    use crate::db;
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::Result;
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::JsonUtil;
    use nostr::Kind;
    use nostr::Tag;
    use nostr::Timestamp;

    async fn make_nip98_event(keys: &Keys, url: &str, method: &str) -> String {
        let event = EventBuilder::new(Kind::from_u16(27235), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign(keys)
            .await
            .unwrap();
        BASE64.encode(event.as_json().as_bytes())
    }

    #[test]
    async fn create_token_no_auth_header() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/nostr").service(super::create_token)),
        )
        .await;

        let req = TestRequest::post().uri("/nostr/token").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn create_token_no_linked_account() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let url = "http://localhost:8080/nostr/token";
        let b64 = make_nip98_event(&keys, url, "POST").await;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/nostr").service(super::create_token)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/nostr/token")
            .insert_header((header::AUTHORIZATION, format!("Nostr {b64}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn create_token_success() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let pubkey_hex = keys.public_key().to_hex();

        // Create user and link npub
        let user = db::main::user::queries::insert("nostr_user", "", &pool).await?;
        db::main::user::queries::set_roles(user.id, &[Role::User], &pool).await?;
        db::main::user::queries::set_npub(user.id, Some(pubkey_hex), &pool).await?;

        let url = "http://localhost:8080/nostr/token";
        let b64 = make_nip98_event(&keys, url, "POST").await;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/nostr").service(super::create_token)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/nostr/token")
            .insert_header((header::AUTHORIZATION, format!("Nostr {b64}")))
            .to_request();
        let res: super::TokenResponse = test::call_and_read_body_json(&app, req).await;
        assert!(!res.token.is_empty());
        Ok(())
    }
}
