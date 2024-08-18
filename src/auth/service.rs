use super::Token;
use crate::{discord, Error};
use actix_web::{http::header::HeaderMap, HttpRequest};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use tracing::warn;

#[cfg(test)]
pub async fn mock_token(secret: &str, pool: &Pool) -> Token {
    let secret = secret.to_string();
    pool.get()
        .await
        .unwrap()
        .interact(move |conn| Token::insert("test", &secret, conn))
        .await
        .unwrap()
        .unwrap()
}

pub async fn check(req: &HttpRequest, pool: &Pool) -> Result<Token, Error> {
    let headers = req.headers().clone();
    let guard = pool.get().await.unwrap();
    let conn = guard.lock().unwrap();
    get_admin_token(&conn, &headers).await
}

pub async fn get_admin_token(db: &Connection, headers: &HeaderMap) -> Result<Token, Error> {
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
            return Ok(token);
        }
        None => {
            let log_message = "Someone tried and failed to access admin API";
            warn!(log_message);
            discord::send_message_to_channel(log_message, discord::CHANNEL_API).await;
            Err(Error::HttpUnauthorized("Invalid token".into()))?
        }
    }
}

#[cfg(test)]
mod tests {
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
    use deadpool_sqlite::Pool;
    use std::sync::Arc;

    #[actix_web::test]
    async fn no_header() -> Result<()> {
        let state = mock_state().await;
        super::mock_token("test", &state.pool).await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        super::mock_token("test", &state.pool).await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
    async fn get(req: HttpRequest, pool: Data<Arc<Pool>>) -> Result<impl Responder, Error> {
        super::check(&req, &pool).await?;
        Ok(Response::ok())
    }
}
