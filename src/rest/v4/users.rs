use crate::db::main::user::schema::User;
use crate::db::main::MainPool;
use crate::db::{self, main::user::schema::Role};
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::nostr_auth::NostrProof;
use crate::rest::v4::top_editors::validate_limit;
use crate::Error;
use actix_web::delete;
use actix_web::get;
use actix_web::http::header;
use actix_web::patch;
use actix_web::post;
use actix_web::put;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
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
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SavedPlace {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SavedArea {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct MeResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub roles: Vec<String>,
    pub saved_places: Vec<SavedPlace>,
    pub saved_areas: Vec<SavedArea>,
    /// Area ids the user is restricted to when acting as an event manager.
    /// Empty means unrestricted.
    #[ts(type = "Array<number>")]
    pub geofence: Vec<i64>,
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
            geofence: user.geofence.clone(),
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
        geofence: user.geofence,
        npub: user.npub,
    }))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "CreateUserArgs")]
pub struct PostArgs {
    pub name: Option<String>,
    pub password: String,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export, rename = "CreateUserResponse")]
pub struct PostResponse {
    #[ts(type = "number")]
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
pub struct GetUsersArgs {
    /// Case-insensitive substring to match against usernames. An empty value
    /// lists non-deleted users up to `limit`.
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "UserSearchResult")]
pub struct UserSearchResult {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub roles: Vec<String>,
    #[ts(type = "string")]
    pub created_at: String,
    /// Area ids the user is restricted to when acting as an event manager.
    /// Empty means unrestricted (same meaning as on `GET /users/me`).
    #[ts(type = "Array<number>")]
    pub geofence: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub npub: Option<String>,
}

impl From<User> for UserSearchResult {
    fn from(user: User) -> Self {
        UserSearchResult {
            id: user.id,
            name: user.name,
            roles: user.roles.iter().map(|role| role.to_string()).collect(),
            created_at: user.created_at,
            geofence: user.geofence,
            npub: user.npub,
        }
    }
}

/// `GET /v4/users?query=<substring>&limit=<n>`
///
/// Admin/root-only user lookup. Returns users whose `name` contains `query`
/// (case-insensitive substring), ordered by name. Deleted users are excluded
/// and `%`/`_` in `query` are treated literally. `limit` defaults to 100 and
/// is capped at 1000.
#[get("")]
pub async fn get(
    auth: Auth,
    args: Query<GetUsersArgs>,
    pool: Data<MainPool>,
) -> Result<Json<Vec<UserSearchResult>>, RestApiError> {
    auth.user.as_ref().ok_or_else(RestApiError::unauthorized)?;
    let is_privileged = auth
        .effective_roles()
        .iter()
        .any(|role| matches!(role, Role::Admin | Role::Root));
    if !is_privileged {
        return Err(RestApiError::forbidden());
    }

    let limit = validate_limit(args.limit)?;
    let users = db::main::user::queries::select_by_name_like(&args.query, limit, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(
        users.into_iter().map(UserSearchResult::from).collect(),
    ))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PatchUserArgs")]
pub struct PatchUserArgs {
    /// Replacement role set. Omit to leave roles untouched.
    #[serde(default)]
    #[ts(optional)]
    pub roles: Option<Vec<String>>,
    /// Replacement geofence (area ids). Omit to leave the geofence untouched.
    #[serde(default)]
    #[ts(optional, type = "Array<number>")]
    pub geofence: Option<Vec<i64>>,
}

/// Roles an admin is allowed to grant/revoke. Everything else must stay as-is.
const ADMIN_MANAGED_ROLES: [Role; 2] = [Role::EventManager, Role::AreaManager];

/// Validated result of [`authorize_update`]: the (possibly unchanged) roles and
/// geofence to persist.
struct AuthorizedUpdate {
    roles: Option<Vec<Role>>,
    geofence: Option<Vec<i64>>,
}

fn same_role_set(a: &[Role], b: &[Role]) -> bool {
    a.len() == b.len() && a.iter().all(|role| b.contains(role))
}

/// True when the only difference between `existing` and `requested` is the
/// membership of admin-managed roles (event_manager / area_manager).
fn only_admin_managed_roles_differ(existing: &[Role], requested: &[Role]) -> bool {
    existing
        .iter()
        .chain(requested.iter())
        .filter(|role| !ADMIN_MANAGED_ROLES.contains(role))
        .all(|role| existing.contains(role) == requested.contains(role))
}

/// Applies the role/geofence update policy for a root or admin caller.
///
/// - Root: may update any non-root user's roles (up to, but never `root`) and
///   geofence; may update its own geofence but never its own roles; may not
///   touch another root at all.
/// - Admin: may add/remove `event_manager` / `area_manager` for non-admin,
///   non-root users, and may set the geofence of any target except another
///   admin or a root (their own included). Admins can never change their own
///   roles.
fn authorize_update(
    caller_id: i64,
    caller_roles: &[Role],
    target: &User,
    roles: Option<Vec<Role>>,
    geofence: Option<Vec<i64>>,
) -> Result<AuthorizedUpdate, RestApiError> {
    if roles.is_none() && geofence.is_none() {
        return Err(RestApiError::invalid_input("Nothing to update"));
    }

    let is_root = caller_roles.contains(&Role::Root);
    let is_admin = caller_roles.contains(&Role::Admin);
    let is_self = caller_id == target.id;
    let target_is_root = target.roles.contains(&Role::Root);
    let target_is_admin = target.roles.contains(&Role::Admin);

    if is_root {
        // A root may never modify another root's record.
        if target_is_root && !is_self {
            return Err(RestApiError::forbidden());
        }
        if let Some(ref new_roles) = roles {
            if is_self {
                // A root may update its own geofence but never its own roles.
                if !same_role_set(new_roles, &target.roles) {
                    return Err(RestApiError::forbidden());
                }
            } else if new_roles.contains(&Role::Root) {
                // `admin` is the ceiling — roots cannot create other roots.
                return Err(RestApiError::forbidden());
            }
        }
        return Ok(AuthorizedUpdate { roles, geofence });
    }

    if is_admin {
        // Admins can never touch a root.
        if target_is_root {
            return Err(RestApiError::forbidden());
        }
        // Admins may set their own geofence but never their own roles.
        if is_self {
            if let Some(ref new_roles) = roles {
                if !same_role_set(new_roles, &target.roles) {
                    return Err(RestApiError::forbidden());
                }
            }
            return Ok(AuthorizedUpdate {
                roles: None,
                geofence,
            });
        }
        // Other admins are off-limits entirely.
        if target_is_admin {
            return Err(RestApiError::forbidden());
        }
        if let Some(ref new_roles) = roles {
            if !only_admin_managed_roles_differ(&target.roles, new_roles) {
                return Err(RestApiError::forbidden());
            }
        }
        return Ok(AuthorizedUpdate { roles, geofence });
    }

    Err(RestApiError::forbidden())
}

/// `PATCH /v4/users/{id}`
///
/// Root/admin-only update of another user's roles and/or geofence. See
/// [`authorize_update`] for the exact policy. Returns the updated user in the
/// same shape as `GET /v4/users`.
#[patch("/{id}")]
pub async fn patch(
    auth: Auth,
    path: Path<i64>,
    args: Json<PatchUserArgs>,
    pool: Data<MainPool>,
) -> Result<Json<UserSearchResult>, RestApiError> {
    let caller = auth.user.as_ref().ok_or_else(RestApiError::unauthorized)?;

    let target = db::main::user::queries::select_by_id(*path, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let roles = match &args.roles {
        Some(raw) => Some(
            raw.iter()
                .map(|role| Role::from_str(role).map_err(RestApiError::invalid_input))
                .collect::<Result<Vec<Role>, _>>()?,
        ),
        None => None,
    };

    let authorized = authorize_update(
        caller.id,
        auth.effective_roles(),
        &target,
        roles,
        args.geofence.clone(),
    )?;

    let mut updated = target;
    if let Some(roles) = authorized.roles {
        updated = db::main::user::queries::set_roles(updated.id, &roles, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
    }
    if let Some(geofence) = authorized.geofence {
        updated = db::main::user::queries::set_geofence(updated.id, &geofence, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
    }

    Ok(Json(UserSearchResult::from(updated)))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct CreateTokenArgs {
    pub label: Option<String>,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export)]
pub struct CreateTokenResponse {
    pub token: String,
    pub user: MeResponse,
}

#[derive(Deserialize, Serialize, ts_rs::TS)]
#[ts(export)]
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

#[derive(Deserialize, Serialize, ts_rs::TS)]
#[ts(export)]
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

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
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
            geofence: user.geofence,
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
    use serde_json::json;

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

    // Inserts a user with `roles`, plus an access token with `secret` carrying
    // the same roles, and returns the user id.
    async fn user_with_role_token(
        name: &str,
        roles: &[Role],
        secret: &str,
        pool: &crate::db::main::MainPool,
    ) -> Result<i64> {
        let user = db::main::user::queries::insert(name, "", pool).await?;
        let user = db::main::user::queries::set_roles(user.id, roles, pool).await?;
        db::main::access_token::queries::insert(
            user.id,
            "".into(),
            secret.into(),
            roles.to_vec(),
            pool,
        )
        .await?;
        Ok(user.id)
    }

    #[test]
    async fn get_users_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get().uri("/users?query=na").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn get_users_forbidden_for_regular_user() -> Result<()> {
        let pool = pool();
        user_with_token("regular", None, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .uri("/users?query=reg")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn get_users_admin_matches_usernames_case_insensitively() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        user_with_role_token("Nathan", &[Role::User], "n1", &pool).await?;
        user_with_role_token("natasha", &[Role::User], "n2", &pool).await?;
        user_with_role_token("bob", &[Role::User], "b3", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .uri("/users?query=NA")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        let names: Vec<&str> = res.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(vec!["natasha", "Nathan"], names);
        Ok(())
    }

    #[test]
    async fn get_users_root_can_search() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let carol = user_with_role_token("carol", &[Role::User], "c", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .uri("/users?query=car")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(carol, res[0].id);
        assert_eq!("carol", res[0].name);
        Ok(())
    }

    #[test]
    async fn get_users_exposes_geofence() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let carol = user_with_role_token("carol", &[Role::User], "c", &pool).await?;
        db::main::user::queries::set_geofence(carol, &[3, 7], &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .uri("/users?query=car")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(vec![3, 7], res[0].geofence);
        Ok(())
    }

    #[test]
    async fn get_users_no_match_returns_empty() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        user_with_role_token("alice", &[Role::User], "a", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .uri("/users?query=zzz")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_users_treats_wildcards_literally() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        user_with_role_token("alice", &[Role::User], "a", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .uri("/users?query=%25")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_users_empty_query_lists_users() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        user_with_role_token("alice", &[Role::User], "a", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(get)),
        )
        .await;

        let req = TestRequest::get()
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .uri("/users?query=")
            .to_request();
        let res: Vec<UserSearchResult> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }

    // App with the user search + update endpoints mounted at /users.
    fn users_app(
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
            .service(scope("/users").service(get).service(patch))
    }

    #[test]
    async fn patch_user_unauthenticated_returns_401() -> Result<()> {
        let app = test::init_service(users_app(pool())).await;
        let req = TestRequest::patch()
            .uri("/users/1")
            .set_json(json!({ "geofence": [1] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn patch_user_regular_user_is_forbidden() -> Result<()> {
        let pool = pool();
        let user = user_with_token("regular", None, &pool).await?;
        let app = test::init_service(users_app(pool)).await;
        let req = TestRequest::patch()
            .uri(&format!("/users/{}", user.id))
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .set_json(json!({ "geofence": [1] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn patch_user_root_promotes_to_admin_and_sets_geofence() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "roles": ["user", "admin"], "geofence": [5, 6] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert_eq!(bob, res.id);
        assert!(res.roles.contains(&"admin".to_string()));
        assert_eq!(vec![5, 6], res.geofence);
        Ok(())
    }

    #[test]
    async fn patch_user_root_cannot_grant_root() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "roles": ["user", "root"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn patch_user_root_cannot_edit_another_root() -> Result<()> {
        let pool = pool();
        user_with_role_token("root_a", &[Role::Root], "a-secret", &pool).await?;
        let root_b = user_with_role_token("root_b", &[Role::Root], "b-secret", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        for body in [json!({ "geofence": [1] }), json!({ "roles": ["user"] })] {
            let req = TestRequest::patch()
                .uri(&format!("/users/{root_b}"))
                .insert_header((header::AUTHORIZATION, "Bearer a-secret"))
                .set_json(body)
                .to_request();
            let res = test::call_service(&app, req).await;
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
        }
        Ok(())
    }

    #[test]
    async fn patch_user_root_can_update_own_geofence_but_not_roles() -> Result<()> {
        let pool = pool();
        let root_id = user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{root_id}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "geofence": [9] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert_eq!(vec![9], res.geofence);

        // Changing own roles is rejected...
        let req = TestRequest::patch()
            .uri(&format!("/users/{root_id}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "roles": ["user"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // ...but echoing the current roles back is a no-op that succeeds.
        let req = TestRequest::patch()
            .uri(&format!("/users/{root_id}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "roles": ["root"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[test]
    async fn patch_user_admin_manages_event_manager() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["user", "event_manager"] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert!(res.roles.contains(&"event_manager".to_string()));

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["user"] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert!(!res.roles.contains(&"event_manager".to_string()));
        Ok(())
    }

    #[test]
    async fn patch_user_admin_cannot_grant_admin() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["user", "admin"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn patch_user_admin_cannot_touch_admins_or_roots() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let other_admin = user_with_role_token("admin2", &[Role::Admin], "a2", &pool).await?;
        let root = user_with_role_token("root", &[Role::Root], "r", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        for target in [other_admin, root] {
            let req = TestRequest::patch()
                .uri(&format!("/users/{target}"))
                .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
                .set_json(json!({ "geofence": [1] }))
                .to_request();
            let res = test::call_service(&app, req).await;
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
        }
        Ok(())
    }

    #[test]
    async fn patch_user_admin_can_set_geofence() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "geofence": [2, 4] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert_eq!(vec![2, 4], res.geofence);
        Ok(())
    }

    #[test]
    async fn patch_user_admin_can_set_own_geofence() -> Result<()> {
        let pool = pool();
        let admin = user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{admin}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "geofence": [3, 4] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert_eq!(admin, res.id);
        assert_eq!(vec![3, 4], res.geofence);
        Ok(())
    }

    #[test]
    async fn patch_user_admin_cannot_change_own_roles() -> Result<()> {
        let pool = pool();
        let admin = user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        // A change is rejected...
        let req = TestRequest::patch()
            .uri(&format!("/users/{admin}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["admin", "event_manager"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // ...but echoing the current roles back is a no-op that succeeds.
        let req = TestRequest::patch()
            .uri(&format!("/users/{admin}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["admin"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[test]
    async fn patch_user_admin_can_set_geofence_for_managers() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let em = user_with_role_token("em", &[Role::User, Role::EventManager], "em-secret", &pool)
            .await?;
        let am = user_with_role_token("am", &[Role::User, Role::AreaManager], "am-secret", &pool)
            .await?;
        let app = test::init_service(users_app(pool)).await;

        for target in [em, am] {
            let req = TestRequest::patch()
                .uri(&format!("/users/{target}"))
                .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
                .set_json(json!({ "geofence": [1] }))
                .to_request();
            let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
            assert_eq!(vec![1], res.geofence);
        }
        Ok(())
    }

    #[test]
    async fn patch_user_admin_can_still_demote_a_manager() -> Result<()> {
        let pool = pool();
        user_with_role_token("admin", &[Role::Admin], "admin-secret", &pool).await?;
        let em = user_with_role_token("em", &[Role::User, Role::EventManager], "em-secret", &pool)
            .await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{em}"))
            .insert_header((header::AUTHORIZATION, "Bearer admin-secret"))
            .set_json(json!({ "roles": ["user"] }))
            .to_request();
        let res: UserSearchResult = test::call_and_read_body_json(&app, req).await;
        assert!(!res.roles.contains(&"event_manager".to_string()));
        Ok(())
    }

    #[test]
    async fn patch_user_unknown_returns_404() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri("/users/999999")
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "geofence": [1] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    async fn patch_user_nothing_to_update_returns_400() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({}))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn patch_user_invalid_role_returns_400() -> Result<()> {
        let pool = pool();
        user_with_role_token("root", &[Role::Root], "root-secret", &pool).await?;
        let bob = user_with_role_token("bob", &[Role::User], "b", &pool).await?;
        let app = test::init_service(users_app(pool)).await;

        let req = TestRequest::patch()
            .uri(&format!("/users/{bob}"))
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({ "roles": ["wizard"] }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }
}
