use crate::db::main::user::schema::User;
use crate::db::main::MainPool;
use crate::db::{self, main::user::schema::Role};
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
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
}

impl From<&User> for MeResponse {
    fn from(user: &User) -> Self {
        MeResponse {
            id: user.id,
            name: user.name.clone(),
            roles: user.roles.iter().map(|r| r.to_string()).collect(),
            saved_places: vec![],
            saved_areas: vec![],
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
        },
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};

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
