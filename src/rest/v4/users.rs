use crate::db::main::user::schema::User;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use actix_web::get;
use actix_web::web::Json;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct MeResponse {
    pub id: i64,
    pub name: String,
    pub roles: Vec<String>,
}

impl From<&User> for MeResponse {
    fn from(user: &User) -> Self {
        MeResponse {
            id: user.id,
            name: user.name.clone(),
            roles: user.roles.iter().map(|r| r.to_string()).collect(),
        }
    }
}

#[get("/me")]
pub async fn me(auth: Auth) -> Result<Json<MeResponse>, RestApiError> {
    match auth.user {
        Some(user) => Ok(Json(MeResponse::from(&user))),
        None => Err(RestApiError::unauthorized()),
    }
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
}
