use crate::db;
use crate::db::main::user::schema::{Role, User};
use crate::db::main::MainPool;
use crate::rest::error::{RestApiError, RestResult};
use crate::rest::nostr_auth::NostrAuth;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use names::Generator;
use names::Name;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct AuthNostrResponse {
    pub token: String,
    pub username: String,
    pub npub: String,
}

/// `POST /v4/auth/nostr`
///
/// Exchange a NIP-98 signed event for a BTC Map Bearer token.
///
/// Auth is via `Authorization: Nostr <base64(event)>`. Verification is
/// performed by the [`NostrAuth`](crate::rest::nostr_auth::NostrAuth)
/// extractor against the URL pinned by `ApiBaseUrl` — request headers
/// (`Host`, `X-Forwarded-*`) are not trusted.
///
/// On success, looks up the user by bech32 npub. If no user is linked to
/// the pubkey, a fresh account is auto-created in a single INSERT
/// (generated name, empty password, role `User`, npub set). A token is
/// minted bound to that user and returned alongside `username` and `npub`.
///
/// Concurrency: two simultaneous first-time sign-ins for the same pubkey
/// race on `INSERT INTO user(... npub)`. There is no unique index on
/// `user.npub` yet, so today the loser is recovered by re-selecting on
/// npub (best-effort — a truly simultaneous race could briefly create two
/// rows). If the partial unique index on `user.npub` is added later, the
/// loser fails atomically with a SQLite ConstraintViolation and this handler
/// re-selects the winning row, so exactly one fully-initialized user exists.
/// Any other database error is propagated rather than masked as a lost race.
#[post("/nostr")]
pub async fn auth_nostr(auth: NostrAuth, pool: Data<MainPool>) -> RestResult<AuthNostrResponse> {
    let npub = auth.npub.ok_or_else(RestApiError::unauthorized)?;

    let user = match db::main::user::queries::select_by_npub(npub.clone(), &pool)
        .await
        .map_err(|_| RestApiError::database())?
    {
        Some(u) => u,
        None => create_or_recover(&npub, &pool).await?,
    };

    // Mint with empty token roles so authorization always derives from
    // the user's current roles (see rpc::handler — non-empty token roles
    // override the user's). Otherwise role revocations on the user would
    // not take effect for already-issued Nostr tokens.
    let secret = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(user.id, String::new(), secret.clone(), vec![], &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    Ok(Json(AuthNostrResponse {
        token: secret,
        username: user.name,
        npub,
    }))
}

/// Auto-create a user for an unknown npub. The insert sets `roles`
/// atomically alongside `npub`. Two distinct UNIQUE failures are possible
/// here, and they're handled differently:
///
/// * `user.npub` — lost a race against a concurrent first-time login for
///   the same pubkey. Recover by selecting the winning row so both
///   callers agree on a single, fully-initialized user.
/// * `user.name` — the random `Name::Numbered` generator collided with an
///   existing user. Retry with a fresh name a few times.
///
/// Any other database error (pool, panic, NOT NULL, other UNIQUE indexes,
/// foreign keys, ...) is propagated as a database error rather than
/// silently masked as a lost race.
const NAME_RETRIES: u8 = 5;

async fn create_or_recover(npub: &str, pool: &MainPool) -> Result<User, RestApiError> {
    for _ in 0..NAME_RETRIES {
        let name = Generator::with_naming(Name::Numbered)
            .next()
            .unwrap_or_default();

        match db::main::user::queries::insert_with_npub(
            name,
            String::new(),
            npub,
            &[Role::User],
            pool,
        )
        .await
        {
            Ok(user) => return Ok(user),
            Err(e) if is_unique_violation_on(&e, "user.npub") => {
                return db::main::user::queries::select_by_npub(npub.to_string(), pool)
                    .await
                    .map_err(|_| RestApiError::database())?
                    .ok_or_else(RestApiError::database);
            }
            Err(e) if is_unique_violation_on(&e, "user.name") => continue,
            Err(_) => return Err(RestApiError::database()),
        }
    }
    Err(RestApiError::database())
}

/// True iff `err` is a SQLite `ConstraintViolation` whose message names
/// `target` (e.g. `"user.npub"`). SQLite's UNIQUE failure message has the
/// form `UNIQUE constraint failed: <table>.<col>`, so column-level
/// matching is robust enough without parsing extended error codes.
///
/// Shared with `users::put_nostr`, which maps a `user.npub` violation to a
/// 400 so the identity-link endpoint stays correct if/when a unique index
/// on `user.npub` is added.
pub(crate) fn is_unique_violation_on(err: &crate::Error, target: &str) -> bool {
    matches!(
        err,
        crate::Error::Rusqlite(rusqlite::Error::SqliteFailure(e, msg))
            if e.code == rusqlite::ErrorCode::ConstraintViolation
                && msg.as_deref().is_some_and(|m| m.contains(target))
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use crate::rest::nostr_auth::ApiBaseUrl;
    use crate::Result;
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::nips::nip19::ToBech32;
    use nostr::{JsonUtil, Kind, Tag, Timestamp};

    const BASE_URL: &str = "https://api.example.test";
    const ENDPOINT_PATH: &str = "/v4/auth/nostr";

    fn signed_nip98(keys: &Keys, method: &str) -> String {
        let url = format!("{BASE_URL}{ENDPOINT_PATH}");
        let event = EventBuilder::new(Kind::from_u16(27235), "")
            .tags(vec![
                Tag::parse(["u", &url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(keys)
            .unwrap();
        BASE64.encode(event.as_json().as_bytes())
    }

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
            .app_data(Data::new(ApiBaseUrl(BASE_URL.to_string())))
            .service(scope("/v4").service(scope("/auth").service(auth_nostr)))
    }

    #[actix_web::test]
    async fn missing_header_returns_401() -> Result<()> {
        let app = test::init_service(build_app(Data::new(pool()))).await;
        let req = TestRequest::post().uri(ENDPOINT_PATH).to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[actix_web::test]
    async fn malformed_authorization_returns_401() -> Result<()> {
        let app = test::init_service(build_app(Data::new(pool()))).await;
        let req = TestRequest::post()
            .uri(ENDPOINT_PATH)
            .insert_header((header::AUTHORIZATION, "Bearer not-a-nostr-event"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[actix_web::test]
    async fn known_npub_returns_token_for_existing_user() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();
        let user = db::main::user::queries::insert_with_npub(
            "preexisting_user",
            "",
            &npub,
            &[Role::User],
            &pool,
        )
        .await?;

        let app = test::init_service(build_app(Data::new(pool.clone()))).await;
        let payload = signed_nip98(&keys, "POST");
        let req = TestRequest::post()
            .uri(ENDPOINT_PATH)
            .insert_header((header::AUTHORIZATION, format!("Nostr {payload}")))
            .to_request();
        let res: AuthNostrResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.username, "preexisting_user");
        assert_eq!(res.npub, npub);
        // Token must actually be persisted and bound to the existing user
        let token = db::main::access_token::queries::select_by_secret(res.token, &pool).await?;
        assert_eq!(token.user_id, user.id);
        Ok(())
    }

    #[actix_web::test]
    async fn unknown_npub_auto_creates_user_and_returns_token() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();

        let app = test::init_service(build_app(Data::new(pool.clone()))).await;
        let payload = signed_nip98(&keys, "POST");
        let req = TestRequest::post()
            .uri(ENDPOINT_PATH)
            .insert_header((header::AUTHORIZATION, format!("Nostr {payload}")))
            .to_request();
        let res: AuthNostrResponse = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.npub, npub);
        // The user must exist in the DB with the right npub and role User
        let user = db::main::user::queries::select_by_npub(npub.clone(), &pool)
            .await?
            .expect("auto-created user should be findable by npub");
        assert_eq!(user.name, res.username);
        assert!(user.roles.contains(&Role::User));
        // Token must be bound to that user
        let token = db::main::access_token::queries::select_by_secret(res.token, &pool).await?;
        assert_eq!(token.user_id, user.id);
        Ok(())
    }

    #[actix_web::test]
    async fn unknown_npub_concurrent_inserts_create_exactly_one_user() -> Result<()> {
        // Two parallel select_by_npub calls both return None, then both
        // attempt insert_with_npub; create_or_recover re-selects on the npub
        // so the net effect is one user row. (There is no unique index on
        // user.npub yet, so this exercises the recovery logic, not a
        // DB-enforced constraint.)
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();

        let pool_a = pool.clone();
        let pool_b = pool.clone();
        let npub_a = npub.clone();
        let npub_b = npub.clone();

        let (a, b) = tokio::join!(
            create_or_recover(&npub_a, &pool_a),
            create_or_recover(&npub_b, &pool_b),
        );
        let user_a = a.expect("first call should succeed");
        let user_b = b.expect("second call should succeed (via recover path)");

        // Both calls must agree on the same row, and roles must be set —
        // there is no window where the row exists with empty roles, even
        // for the loser branch which selects the winner's row.
        assert_eq!(user_a.id, user_b.id);
        assert_eq!(user_a.npub, Some(npub.clone()));
        assert!(user_a.roles.contains(&Role::User));
        assert!(user_b.roles.contains(&Role::User));
        Ok(())
    }
}
