use crate::Error;

use super::Token;
use actix_web::{http::header::HeaderMap, HttpRequest};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use std::sync::Arc;
use tracing::warn;

pub struct AuthService {
    pool: Arc<Pool>,
}

impl AuthService {
    pub fn new(pool: &Arc<Pool>) -> Self {
        Self { pool: pool.clone() }
    }

    #[cfg(test)]
    pub async fn mock_token(&self, user_id: i64, secret: &str) -> Token {
        let secret = secret.to_string();
        self.pool
            .get()
            .await
            .unwrap()
            .interact(move |conn| Token::insert(user_id, &secret, conn))
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn check(&self, req: &HttpRequest) -> Result<Token, Error> {
        let headers = req.headers().clone();
        self.pool
            .get()
            .await
            .unwrap()
            .interact(move |conn| get_admin_token(&conn, &headers))
            .await
            .unwrap()
    }
}

pub fn get_admin_token(db: &Connection, headers: &HeaderMap) -> Result<Token, Error> {
    let auth_header = headers
        .get("Authorization")
        .map(|it| it.to_str().unwrap_or(""))
        .unwrap_or("");
    if auth_header.len() == 0 {
        Err(Error::HttpUnauthorized(
            "Authorization header is missing".into(),
        ))?
    }
    let auth_header_parts: Vec<&str> = auth_header.split(" ").collect();
    if auth_header_parts.len() != 2 {
        Err(Error::HttpUnauthorized(
            "Authorization header is invalid".into(),
        ))?
    }
    let secret = auth_header_parts[1];
    let token = Token::select_by_secret(secret, db)?;
    match token {
        Some(token) => {
            warn!(
                admin_channel_message = format!(
                    "Admin API was accessed by https://api.btcmap.org/v2/users/{}",
                    token.user_id
                )
            );
            return Ok(token);
        }
        None => {
            warn!(admin_channel_message = "Someone tried and failed to access admin API");
            Err(Error::HttpUnauthorized("Invalid token".into()))?
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::AuthService;
    use crate::osm::osm::OsmUser;
    use crate::test::mock_state;
    use crate::{Error, Result};
    use actix_web::test::{self, TestRequest};
    use actix_web::HttpRequest;
    use actix_web::{
        dev::Response,
        get,
        web::{scope, Data},
        App, Responder,
    };

    #[actix_web::test]
    async fn no_header() -> Result<()> {
        let state = mock_state().await;
        state.user_repo.insert(1, &OsmUser::mock()).await?;
        state.auth.mock_token(1, "test").await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(401, res.status().as_u16());
        Ok(())
    }

    #[actix_web::test]
    async fn valid_token() -> Result<()> {
        let state = mock_state().await;
        state.user_repo.insert(1, &OsmUser::mock()).await?;
        state.auth.mock_token(1, "test").await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/")
            .append_header(("Authorization", "Bearer test"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(200, res.status().as_u16());
        Ok(())
    }

    #[get("")]
    async fn get(req: HttpRequest, auth: Data<AuthService>) -> Result<impl Responder, Error> {
        auth.check(&req).await?;
        Ok(Response::ok())
    }
}
