use super::Token;
use crate::ApiError;
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

    pub async fn check(&self, req: &HttpRequest) -> Result<Token, ApiError> {
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

pub fn get_admin_token(db: &Connection, headers: &HeaderMap) -> Result<Token, ApiError> {
    let auth_header = headers
        .get("Authorization")
        .map(|it| it.to_str().unwrap_or(""))
        .unwrap_or("");
    if auth_header.len() == 0 {
        return Err(ApiError::new(401, "Authorization header is missing"));
    }
    let auth_header_parts: Vec<&str> = auth_header.split(" ").collect();
    if auth_header_parts.len() != 2 {
        return Err(ApiError::new(401, "Authorization header is invalid"));
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
            return Err(ApiError::new(401, "Invalid token"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db;
    use crate::Result;
    use actix_web::{
        dev::Response,
        get,
        test::{self, TestRequest},
        web::{scope, Data},
        App, Responder,
    };
    use rusqlite::Connection;

    #[actix_web::test]
    async fn no_header() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;
        Token::insert(1, "test", &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;
        Token::insert(1, "test", &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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
    async fn get(req: HttpRequest, db: Data<Connection>) -> Result<impl Responder, ApiError> {
        get_admin_token(&db, &req.headers())?;
        Ok(Response::ok())
    }
}
