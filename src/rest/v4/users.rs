use crate::db::main::user::schema::User;
use crate::db::main::MainPool;
use crate::db::{self, main::user::schema::Role};
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::nostr_auth::NostrProof;
use actix_web::delete;
use actix_web::get;
use actix_web::http::header;
use actix_web::post;
use actix_web::put;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::Argon2;
use argon2::PasswordHash;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use names::Generator;
use names::Name;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct SavedPlace {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SavedArea {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct MeResponse {
    pub id: i64,
    pub name: String,
    pub roles: Vec<String>,
    pub saved_places: Vec<SavedPlace>,
    pub saved_areas: Vec<SavedArea>,
    /// Bech32 npub (`npub1...`) of the Nostr identity linked to this user,
    /// or `null` when no pubkey is linked.
    pub npub: Option<String>,
}

impl From<&User> for MeResponse {
    fn from(user: &User) -> Self {
        MeResponse {
            id: user.id,
            name: user.name.clone(),
            roles: user.roles.iter().map(|r| r.to_string()).collect(),
            saved_places: vec![],
            saved_areas: vec![],
            npub: user.npub.clone(),
        }
    }
}

#[get("/me")]
pub async fn me(auth: Auth, pool: Data<MainPool>) -> Result<Json<MeResponse>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let saved_places = db::main::element::queries::select_by_ids(&user.saved_places, &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .into_iter()
        .map(|e| SavedPlace {
            id: e.id,
            name: e.name(None),
        })
        .collect();
    let saved_areas = db::main::area::queries::select_by_ids(&user.saved_areas, &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .into_iter()
        .map(|a| SavedArea {
            id: a.id,
            name: a.name(),
        })
        .collect();
    Ok(Json(MeResponse {
        id: user.id,
        name: user.name,
        roles: user.roles.iter().map(|r| r.to_string()).collect(),
        saved_places,
        saved_areas,
        npub: user.npub,
    }))
}

#[derive(Deserialize)]
pub struct PostArgs {
    pub name: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct PostResponse {
    pub id: i64,
    pub name: String,
    pub roles: Vec<String>,
}

#[post("")]
pub async fn post(
    args: Json<PostArgs>,
    pool: Data<MainPool>,
) -> Result<Json<PostResponse>, RestApiError> {
    let name = match &args.name {
        Some(n) => n.clone(),
        None => Generator::with_naming(Name::Numbered)
            .next()
            .unwrap_or_default(),
    };
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(args.password.as_bytes(), &salt)
        .map_err(|e| RestApiError::invalid_input(e.to_string()))?
        .to_string();
    let user = db::main::user::queries::insert(&name, password_hash, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let user = db::main::user::queries::set_roles(user.id, &[Role::User], &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(PostResponse {
        id: user.id,
        name: user.name,
        roles: user.roles.into_iter().map(|it| it.to_string()).collect(),
    }))
}

#[derive(Deserialize)]
pub struct CreateTokenArgs {
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub user: MeResponse,
}

#[derive(Deserialize, Serialize)]
pub struct ChangePasswordArgs {
    pub old_password: String,
    pub new_password: String,
}

#[put("/me/password")]
pub async fn change_password(
    auth: Auth,
    args: Json<ChangePasswordArgs>,
    pool: Data<MainPool>,
) -> Result<Json<()>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let old_password_hash = PasswordHash::new(&user.password)
        .map_err(|_| RestApiError::invalid_input("Invalid password hash"))?;
    Argon2::default()
        .verify_password(args.old_password.as_bytes(), &old_password_hash)
        .map_err(|_| RestApiError::invalid_input("Invalid old password"))?;
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(args.new_password.as_bytes(), &salt)
        .map_err(|e| RestApiError::invalid_input(e.to_string()))?
        .to_string();
    db::main::user::queries::set_password(user.id, password_hash, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(()))
}

#[derive(Deserialize, Serialize)]
pub struct UpdateUsernameArgs {
    pub username: String,
}

#[put("/me/username")]
pub async fn update_username(
    auth: Auth,
    args: Json<UpdateUsernameArgs>,
    pool: Data<MainPool>,
) -> Result<Json<MeResponse>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let updated_user = db::main::user::queries::set_name(user.id, &args.username, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(MeResponse::from(&updated_user)))
}

#[derive(Serialize, Deserialize)]
pub struct NostrIdentityResponse {
    /// Bech32 npub (`npub1...`) currently linked to the account, or `null`.
    pub npub: Option<String>,
}

/// `GET /v4/users/me/nostr`
///
/// Returns the Nostr pubkey currently linked to the authenticated account
/// (or `null`). A thin read of the same `npub` exposed on `GET /me`, kept
/// as a dedicated sub-resource so a client can poll just the link state.
#[get("/me/nostr")]
pub async fn get_nostr(auth: Auth) -> Result<Json<NostrIdentityResponse>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    Ok(Json(NostrIdentityResponse { npub: user.npub }))
}

/// `PUT /v4/users/me/nostr`
///
/// Links (or replaces) the Nostr pubkey on the authenticated account.
/// Requires TWO credentials: a Bearer token (`Authorization`, via [`Auth`])
/// to say *which account*, and a NIP-98 proof (`X-Nostr-Authorization`, via
/// [`NostrProof`]) to prove control of the pubkey being linked. The request
/// body is empty; the proof event must sign `u = <ApiBaseUrl>/v4/users/me/nostr`
/// with method `PUT`.
///
/// Conflict handling is application-level: if the proven npub is already
/// linked to a *different* account, returns 400. Idempotent: re-linking the
/// npub already on this account returns 200.
///
/// NOTE (concurrency): there is no UNIQUE index on `user.npub` yet, so the
/// `select_by_npub` check and the `set_npub` write are not atomic — two
/// concurrent PUTs linking the same npub to two different accounts could
/// both pass the check (TOCTOU). The conflict check is therefore the
/// best-effort guard for today's schema. The write is *also* wrapped so
/// that a `user.npub` UNIQUE violation maps to 400 rather than 500: this is
/// a no-op against the current schema (no index can fire), but means the
/// endpoint becomes race-safe automatically if the maintainer-owned partial
/// unique index on `user.npub` is added later — no code change required, and
/// the race-loser gets the same 400 as the check-rejected path. Do not add
/// that index here.
#[put("/me/nostr")]
pub async fn put_nostr(
    auth: Auth,
    proof: NostrProof,
    pool: Data<MainPool>,
) -> Result<Json<NostrIdentityResponse>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let npub = proof.npub.ok_or_else(RestApiError::unauthorized)?;

    // Refuse to steal a pubkey already linked to someone else. Linking the
    // npub this account already has is a no-op that still returns 200.
    if let Some(existing) = db::main::user::queries::select_by_npub(npub.clone(), &pool)
        .await
        .map_err(|_| RestApiError::database())?
    {
        if existing.id != user.id {
            return Err(npub_conflict());
        }
    }

    db::main::user::queries::set_npub(user.id, Some(npub.clone()), &pool)
        .await
        .map_err(|e| {
            // Backstop for the TOCTOU window: if a unique index on
            // `user.npub` exists and the concurrent loser hits it, surface
            // the documented 400 instead of a generic 500.
            if crate::rest::v4::nostr::is_unique_violation_on(&e, "user.npub") {
                npub_conflict()
            } else {
                RestApiError::database()
            }
        })?;

    Ok(Json(NostrIdentityResponse { npub: Some(npub) }))
}

/// 400 returned when the proven npub is already linked to a different account
/// (either caught by the pre-check or by a UNIQUE violation on write).
fn npub_conflict() -> RestApiError {
    RestApiError::invalid_input("npub already linked to another account")
}

/// `DELETE /v4/users/me/nostr`
///
/// Clears the Nostr pubkey linked to the authenticated account. Requires
/// only account auth (Bearer) — removing your own link needs no NIP-98
/// proof. Idempotent: succeeds with `npub: null` even if nothing was
/// linked.
#[delete("/me/nostr")]
pub async fn delete_nostr(
    auth: Auth,
    pool: Data<MainPool>,
) -> Result<Json<NostrIdentityResponse>, RestApiError> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    db::main::user::queries::set_npub(user.id, None, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(NostrIdentityResponse { npub: None }))
}

#[post("/{username}/tokens")]
pub async fn create_token(
    req: actix_web::HttpRequest,
    username: web::Path<String>,
    args: Json<CreateTokenArgs>,
    pool: Data<MainPool>,
) -> Result<Json<CreateTokenResponse>, RestApiError> {
    let password = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(String::from)
        .ok_or_else(RestApiError::unauthorized)?;

    let user = db::main::user::queries::select_by_name(&*username, &pool)
        .await
        .map_err(|_| RestApiError::unauthorized())?;

    // Users provisioned via Nostr (NIP-98) have no password set. Block
    // password auth for them explicitly so an empty bearer cannot be
    // mistaken for a credential. PHC parsing of an empty hash already
    // fails today, but make the intent explicit.
    if user.password.is_empty() {
        return Err(RestApiError::unauthorized());
    }

    let password_hash = PasswordHash::new(&user.password)
        .map_err(|_| RestApiError::invalid_input("Invalid password hash"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|_| RestApiError::invalid_input("Invalid credentials"))?;

    let token = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(
        user.id,
        args.label.clone().unwrap_or_default(),
        token.clone(),
        vec![],
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let saved_places = db::main::element::queries::select_by_ids(&user.saved_places, &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .into_iter()
        .map(|e| SavedPlace {
            id: e.id,
            name: e.name(None),
        })
        .collect();
    let saved_areas = db::main::area::queries::select_by_ids(&user.saved_areas, &pool)
        .await
        .map_err(|_| RestApiError::database())?
        .into_iter()
        .map(|a| SavedArea {
            id: a.id,
            name: a.name(),
        })
        .collect();

    Ok(Json(CreateTokenResponse {
        token,
        user: MeResponse {
            id: user.id,
            name: user.name,
            roles: user.roles.iter().map(|r| r.to_string()).collect(),
            saved_places,
            saved_areas,
            npub: user.npub,
        },
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::rest::nostr_auth::{ApiBaseUrl, X_NOSTR_AUTHORIZATION};
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::nips::nip19::ToBech32;
    use nostr::{JsonUtil, Kind, Tag, Timestamp};

    // Trusted base URL the NIP-98 `u` tag must bind to in PUT /me/nostr tests.
    const BASE: &str = "https://api.example.test";

    // Base64-encoded NIP-98 event signing `url` with `method`, for the
    // `X-Nostr-Authorization` header. Mirrors the helper in nostr.rs/nostr_auth.rs.
    fn signed_nip98(keys: &Keys, url: &str, method: &str) -> String {
        let event = EventBuilder::new(Kind::from_u16(27235), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(keys)
            .unwrap();
        BASE64.encode(event.as_json().as_bytes())
    }

    #[test]
    async fn me_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(me)),
        )
        .await;

        let req = TestRequest::get().uri("/users/me").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn me_authenticated_returns_user() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("test_user", "", &pool).await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(me)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me")
            .to_request();
        let res: MeResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, user.id);
        assert_eq!(res.name, "test_user");
        // A password-only user has no linked Nostr identity.
        assert_eq!(res.npub, None);
        Ok(())
    }

    #[test]
    async fn me_returns_linked_npub() -> Result<()> {
        let pool = pool();
        let npub = "npub1example".to_string();
        let user = db::main::user::queries::insert_with_npub(
            "nostr_user",
            "",
            &npub,
            &[Role::User],
            &pool,
        )
        .await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(me)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me")
            .to_request();
        let res: MeResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, Some(npub));
        Ok(())
    }

    // Inserts a user (optionally with an npub) plus an access token "secret"
    // bound to it, returning the user. Mirrors the inline setup used by the
    // `me` tests above.
    async fn user_with_token(
        name: &str,
        npub: Option<&str>,
        pool: &crate::db::main::MainPool,
    ) -> Result<User> {
        let user = match npub {
            Some(npub) => {
                db::main::user::queries::insert_with_npub(name, "", npub, &[Role::User], pool)
                    .await?
            }
            None => db::main::user::queries::insert(name, "", pool).await?,
        };
        db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::User],
            pool,
        )
        .await?;
        Ok(user)
    }

    #[test]
    async fn get_nostr_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(get_nostr)),
        )
        .await;

        let req = TestRequest::get().uri("/users/me/nostr").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn get_nostr_returns_null_when_unlinked() -> Result<()> {
        let pool = pool();
        user_with_token("plain_user", None, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get_nostr)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/nostr")
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, None);
        Ok(())
    }

    #[test]
    async fn get_nostr_returns_linked_npub() -> Result<()> {
        let pool = pool();
        user_with_token("nostr_user", Some("npub1example"), &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get_nostr)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/nostr")
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, Some("npub1example".to_string()));
        Ok(())
    }

    #[test]
    async fn delete_nostr_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(delete_nostr)),
        )
        .await;

        let req = TestRequest::delete().uri("/users/me/nostr").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn delete_nostr_clears_link() -> Result<()> {
        let pool = pool();
        let user = user_with_token("nostr_user", Some("npub1example"), &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/users").service(delete_nostr)),
        )
        .await;

        let req = TestRequest::delete()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/nostr")
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, None);

        // The link is actually gone from the database.
        let reloaded = db::main::user::queries::select_by_id(user.id, &pool).await?;
        assert_eq!(reloaded.npub, None);
        Ok(())
    }

    #[test]
    async fn delete_nostr_idempotent_when_unlinked() -> Result<()> {
        let pool = pool();
        user_with_token("plain_user", None, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(delete_nostr)),
        )
        .await;

        let req = TestRequest::delete()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/nostr")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    // Builds the PUT /me/nostr test app: pool + the ApiBaseUrl the NIP-98
    // proof binds to, mounted at /users so the signed `u` is BASE/users/me/nostr.
    fn put_app(
        pool: crate::db::main::MainPool,
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
            .app_data(Data::new(pool))
            .app_data(Data::new(ApiBaseUrl(BASE.to_string())))
            .service(scope("/users").service(put_nostr))
    }

    const PUT_URL: &str = "/users/me/nostr";

    #[test]
    async fn put_nostr_links_pubkey() -> Result<()> {
        let pool = pool();
        let user = user_with_token("plain_user", None, &pool).await?;
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();
        let proof = signed_nip98(&keys, &format!("{BASE}{PUT_URL}"), "PUT");

        let app = test::init_service(put_app(pool.clone())).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, Some(npub.clone()));

        let reloaded = db::main::user::queries::select_by_id(user.id, &pool).await?;
        assert_eq!(reloaded.npub, Some(npub));
        Ok(())
    }

    #[test]
    async fn put_nostr_replaces_existing_link() -> Result<()> {
        let pool = pool();
        let old_npub = Keys::generate().public_key().to_bech32().unwrap();
        let user = user_with_token("nostr_user", Some(&old_npub), &pool).await?;
        let keys = Keys::generate();
        let new_npub = keys.public_key().to_bech32().unwrap();
        let proof = signed_nip98(&keys, &format!("{BASE}{PUT_URL}"), "PUT");

        let app = test::init_service(put_app(pool.clone())).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, Some(new_npub.clone()));

        let reloaded = db::main::user::queries::select_by_id(user.id, &pool).await?;
        assert_eq!(reloaded.npub, Some(new_npub));
        Ok(())
    }

    #[test]
    async fn put_nostr_idempotent_when_same_user() -> Result<()> {
        // The account already owns this npub; re-linking it returns 200.
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();
        let user = user_with_token("nostr_user", Some(&npub), &pool).await?;
        let proof = signed_nip98(&keys, &format!("{BASE}{PUT_URL}"), "PUT");

        let app = test::init_service(put_app(pool.clone())).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res: NostrIdentityResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.npub, Some(npub.clone()));

        // The link is unchanged, not cleared.
        let reloaded = db::main::user::queries::select_by_id(user.id, &pool).await?;
        assert_eq!(reloaded.npub, Some(npub));
        Ok(())
    }

    #[test]
    async fn put_nostr_conflict_returns_400() -> Result<()> {
        // npub is already linked to a different account.
        let pool = pool();
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();
        // Account A owns the pubkey.
        let owner =
            db::main::user::queries::insert_with_npub("owner", "", &npub, &[Role::User], &pool)
                .await?;
        // Account B (the caller) tries to claim it.
        let claimer = user_with_token("claimer", None, &pool).await?;
        let proof = signed_nip98(&keys, &format!("{BASE}{PUT_URL}"), "PUT");

        let app = test::init_service(put_app(pool.clone())).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // The link was not moved: claimer stays unlinked, owner keeps it.
        let claimer_after = db::main::user::queries::select_by_id(claimer.id, &pool).await?;
        assert_eq!(claimer_after.npub, None);
        let owner_after = db::main::user::queries::select_by_id(owner.id, &pool).await?;
        assert_eq!(owner_after.npub, Some(npub));
        Ok(())
    }

    #[test]
    async fn put_nostr_missing_bearer_returns_401() -> Result<()> {
        let pool = pool();
        let keys = Keys::generate();
        let proof = signed_nip98(&keys, &format!("{BASE}{PUT_URL}"), "PUT");

        let app = test::init_service(put_app(pool)).await;
        let req = TestRequest::put()
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn put_nostr_missing_proof_returns_401() -> Result<()> {
        let pool = pool();
        user_with_token("plain_user", None, &pool).await?;

        let app = test::init_service(put_app(pool)).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri(PUT_URL)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn put_nostr_proof_for_wrong_url_returns_401() -> Result<()> {
        let pool = pool();
        user_with_token("plain_user", None, &pool).await?;
        let keys = Keys::generate();
        // Proof signs a different path than the request targets.
        let proof = signed_nip98(&keys, &format!("{BASE}/users/me/different"), "PUT");

        let app = test::init_service(put_app(pool)).await;
        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .insert_header((X_NOSTR_AUTHORIZATION, format!("Nostr {proof}")))
            .uri(PUT_URL)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    fn make_password_hash(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    #[test]
    async fn change_password_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(change_password)),
        )
        .await;

        let req = TestRequest::put()
            .uri("/users/me/password")
            .set_json(ChangePasswordArgs {
                old_password: "old".into(),
                new_password: "new".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn change_password_success() -> Result<()> {
        let pool = pool();
        let old_password_hash = make_password_hash("old_password");
        let user = db::main::user::queries::insert("test_user", &old_password_hash, &pool).await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/users").service(change_password)),
        )
        .await;

        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/password")
            .set_json(ChangePasswordArgs {
                old_password: "old_password".into(),
                new_password: "new_password".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let updated_user = db::main::user::queries::select_by_id(user.id, &pool).await?;
        let updated_hash = PasswordHash::new(&updated_user.password).unwrap();
        assert!(Argon2::default()
            .verify_password("new_password".as_bytes(), &updated_hash)
            .is_ok());
        Ok(())
    }

    #[test]
    async fn change_password_wrong_old_password_returns_400() -> Result<()> {
        let pool = pool();
        let old_password_hash = make_password_hash("correct_password");
        let user = db::main::user::queries::insert("test_user", &old_password_hash, &pool).await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(change_password)),
        )
        .await;

        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/password")
            .set_json(ChangePasswordArgs {
                old_password: "wrong_password".into(),
                new_password: "new_password".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn update_username_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(update_username)),
        )
        .await;

        let req = TestRequest::put()
            .uri("/users/me/username")
            .set_json(UpdateUsernameArgs {
                username: "new_name".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn update_username_success() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("old_name", "", &pool).await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(update_username)),
        )
        .await;

        let req = TestRequest::put()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users/me/username")
            .set_json(UpdateUsernameArgs {
                username: "new_name".into(),
            })
            .to_request();
        let res: MeResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, user.id);
        assert_eq!(res.name, "new_name");
        Ok(())
    }
}
