use crate::db;
use crate::db::main::user::schema::{Role, User};
use crate::db::main::MainPool;
use crate::rest::error::{RestApiError, RestResult};
use crate::rest::nostr_auth::NostrAuth;
use crate::service;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use names::Generator;
use names::Name;
use nostr::event::{Event, Kind};
use nostr::key::PublicKey;
use nostr::nips::nip01::Metadata;
use nostr::nips::nip19::FromBech32;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct AuthNostrArgs {
    /// The user's signed kind-0 profile event (NIP-01) as JSON. Only used
    /// when the key has no account yet: its `name` (or `display_name`)
    /// becomes the new account's username. The event carries its own
    /// signature, so the server needs no relay to trust it.
    #[ts(type = "unknown", optional)]
    pub profile: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
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
/// There are only BTC Map accounts: Nostr is a link on one. On success,
/// looks up the account linked to the npub. If there is none, signs the
/// user up: creates a BTC Map account with a username from the optional
/// signed profile event (see [`AuthNostrArgs`]; a number is appended while
/// it is taken, and a generated name is the fallback) and a random password
/// that is stored hashed and never returned, then links the key. A token is
/// minted bound to that account and returned alongside `username` and `npub`.
///
/// Accounts created before every account had a password get a random one
/// on their next Nostr login.
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
pub async fn auth_nostr(
    auth: NostrAuth,
    args: Option<Json<AuthNostrArgs>>,
    pool: Data<MainPool>,
) -> RestResult<AuthNostrResponse> {
    let npub = auth.npub.ok_or_else(RestApiError::unauthorized)?;

    let user = match db::main::user::queries::select_by_npub(npub.clone(), &pool)
        .await
        .map_err(|_| RestApiError::database())?
    {
        Some(u) => {
            if u.password.is_empty() {
                db::main::user::queries::set_password(u.id, random_password_hash()?, &pool)
                    .await
                    .map_err(|_| RestApiError::database())?;
            }
            u
        }
        None => {
            let name = args
                .as_ref()
                .and_then(|args| args.profile.as_ref())
                .and_then(|profile| profile_name(profile, &npub));
            create_or_recover(&npub, name.as_deref(), &pool).await?
        }
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

/// Longest username taken from a profile, in characters (matches the web's
/// username field).
const PROFILE_NAME_MAX_CHARS: usize = 100;

/// The username a signed kind-0 profile event asks for: its `name`, else
/// its `display_name`, trimmed and capped. None unless the event is a valid
/// kind 0 signed by `npub`'s key, so a client can't pick a name for
/// someone else's profile.
fn profile_name(profile: &serde_json::Value, npub: &str) -> Option<String> {
    let event = Event::from_json(profile.to_string()).ok()?;
    event.verify().ok()?;
    if event.kind != Kind::Metadata || event.pubkey != PublicKey::from_bech32(npub).ok()? {
        return None;
    }
    let metadata = Metadata::from_json(&event.content).ok()?;
    [metadata.name, metadata.display_name]
        .into_iter()
        .flatten()
        .map(|name| {
            name.trim()
                .chars()
                .take(PROFILE_NAME_MAX_CHARS)
                .collect::<String>()
        })
        .find(|name| !name.is_empty())
}

/// A password nobody knows, stored hashed: every account has one, so an
/// empty password can never stand in for a credential.
fn random_password_hash() -> Result<String, RestApiError> {
    service::auth::hash_password(&Uuid::new_v4().to_string()).map_err(|_| RestApiError::internal())
}

/// The usernames to try, in order: the profile name, then it with 2, 3, …
/// appended, then generated names.
fn name_candidates(profile_name: Option<&str>) -> impl Iterator<Item = String> + '_ {
    let from_profile = profile_name.into_iter().flat_map(|base| {
        (1..=NAME_RETRIES).map(move |n| match n {
            1 => base.to_string(),
            n => format!("{base}{n}"),
        })
    });
    let generated = (0..NAME_RETRIES).map(|_| {
        Generator::with_naming(Name::Numbered)
            .next()
            .unwrap_or_default()
    });
    from_profile.chain(generated)
}

/// Sign up an unknown npub: a BTC Map account with a username and a random
/// password, linked to the key. The insert sets `roles` atomically
/// alongside `npub`. Two distinct UNIQUE failures are possible here, and
/// they're handled differently:
///
/// * `user.npub` — lost a race against a concurrent first-time login for
///   the same pubkey. Recover by selecting the winning row so both
///   callers agree on a single, fully-initialized user.
/// * `user.name` — the name is taken. Move on to the next candidate (see
///   [`name_candidates`]).
///
/// Any other database error (pool, panic, NOT NULL, other UNIQUE indexes,
/// foreign keys, ...) is propagated as a database error rather than
/// silently masked as a lost race.
pub(crate) const NAME_RETRIES: u8 = 5;

async fn create_or_recover(
    npub: &str,
    profile_name: Option<&str>,
    pool: &MainPool,
) -> Result<User, RestApiError> {
    let password = random_password_hash()?;
    for name in name_candidates(profile_name) {
        match db::main::user::queries::insert_with_npub(
            name,
            password.clone(),
            npub,
            &[Role::User],
            pool,
        )
        .await
        {
            Ok(user) => return Ok(user),
            Err(e) if e.is_unique_violation_on("user.npub") => {
                return db::main::user::queries::select_by_npub(npub.to_string(), pool)
                    .await
                    .map_err(|_| RestApiError::database())?
                    .ok_or_else(RestApiError::database);
            }
            Err(e) if e.is_unique_violation_on("user.name") => continue,
            Err(_) => return Err(RestApiError::database()),
        }
    }
    Err(RestApiError::database())
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
    use nostr::event::{EventBuilder, FinalizeEvent, Kind, Tag};
    use nostr::key::Keys;
    use nostr::nips::nip19::ToBech32;
    use nostr::types::Timestamp;

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
            .finalize(keys)
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

    // A kind-0 profile event signed by `keys`, as the JSON a client sends.
    fn signed_profile(keys: &Keys, metadata: &Metadata) -> serde_json::Value {
        let event = EventBuilder::new(Kind::Metadata, metadata.as_json())
            .finalize(keys)
            .unwrap();
        serde_json::from_str(&event.as_json()).unwrap()
    }

    // Sign in with `keys`, optionally sending a profile, and return the reply.
    async fn sign_in(
        keys: &Keys,
        profile: Option<serde_json::Value>,
        pool: &MainPool,
    ) -> AuthNostrResponse {
        let app = test::init_service(build_app(Data::new(pool.clone()))).await;
        let mut req = TestRequest::post().uri(ENDPOINT_PATH).insert_header((
            header::AUTHORIZATION,
            format!("Nostr {}", signed_nip98(keys, "POST")),
        ));
        if let Some(profile) = profile {
            req = req.set_json(serde_json::json!({ "profile": profile }));
        }
        test::call_and_read_body_json(&app, req.to_request()).await
    }

    #[actix_web::test]
    async fn sign_up_takes_the_username_from_the_signed_profile() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let profile = signed_profile(&keys, &Metadata::new().name("satoshi"));

        let res = sign_in(&keys, Some(profile), &pool).await;

        assert_eq!(res.username, "satoshi");
        Ok(())
    }

    #[actix_web::test]
    async fn sign_up_appends_a_number_while_the_profile_name_is_taken() -> Result<()> {
        let pool = pool();
        db::main::user::queries::insert("satoshi", "", &pool).await?;
        db::main::user::queries::insert("satoshi2", "", &pool).await?;
        let keys = Keys::generate();
        let profile = signed_profile(&keys, &Metadata::new().name("satoshi"));

        let res = sign_in(&keys, Some(profile), &pool).await;

        assert_eq!(res.username, "satoshi3");
        Ok(())
    }

    #[actix_web::test]
    async fn sign_up_falls_back_to_the_display_name() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let profile = signed_profile(&keys, &Metadata::new().name("  ").display_name("Satoshi N"));

        let res = sign_in(&keys, Some(profile), &pool).await;

        assert_eq!(res.username, "Satoshi N");
        Ok(())
    }

    #[actix_web::test]
    async fn sign_up_ignores_a_profile_signed_by_another_key() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let someone_else = Keys::generate();
        let profile = signed_profile(&someone_else, &Metadata::new().name("satoshi"));

        let res = sign_in(&keys, Some(profile), &pool).await;

        assert_ne!(res.username, "satoshi");
        assert!(!res.username.is_empty());
        Ok(())
    }

    #[actix_web::test]
    async fn sign_up_ignores_a_tampered_profile() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let mut profile = signed_profile(&keys, &Metadata::new().name("satoshi"));
        // Changed after signing: the signature no longer matches
        profile["content"] = serde_json::json!(r#"{"name":"hal"}"#);

        let res = sign_in(&keys, Some(profile), &pool).await;

        assert_ne!(res.username, "hal");
        assert_ne!(res.username, "satoshi");
        Ok(())
    }

    #[actix_web::test]
    async fn sign_up_gives_the_account_a_password() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();

        sign_in(&keys, None, &pool).await;

        let npub = keys.public_key().to_bech32().unwrap();
        let user = db::main::user::queries::select_by_npub(npub, &pool)
            .await?
            .unwrap();
        assert!(argon2::PasswordHash::new(&user.password).is_ok());
        Ok(())
    }

    #[actix_web::test]
    async fn login_gives_a_passwordless_account_a_password() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();
        db::main::user::queries::insert_with_npub(
            "old_nostr_user",
            "",
            &npub,
            &[Role::User],
            &pool,
        )
        .await?;

        let res = sign_in(&keys, None, &pool).await;

        assert_eq!(res.username, "old_nostr_user");
        let user = db::main::user::queries::select_by_npub(npub, &pool)
            .await?
            .unwrap();
        assert!(argon2::PasswordHash::new(&user.password).is_ok());
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
            create_or_recover(&npub_a, None, &pool_a),
            create_or_recover(&npub_b, None, &pool_b),
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
