use crate::ApiError;
use actix_web::HttpRequest;
use std::env;

pub fn is_from_admin(req: &HttpRequest) -> Result<(), ApiError> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or("".to_string());

    if admin_token.len() == 0 {
        return Err(ApiError::new(401, "Admin token is not set"));
    }

    let auth_header = req
        .headers()
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

    if auth_header_parts[1] != admin_token {
        return Err(ApiError::new(401, "Invalid token"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::command::db;
    use actix_web::{
        dev::Response,
        get,
        test::{self, TestRequest},
        web::{scope, Data},
        App, Responder,
    };
    use std::sync::atomic::Ordering;

    #[actix_web::test]
    async fn no_header() {
        let admin_token = "test";
        env::set_var("ADMIN_TOKEN", admin_token);
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(401, res.status().as_u16())
    }

    #[get("")]
    async fn get(req: HttpRequest) -> Result<impl Responder, ApiError> {
        is_from_admin(&req)?;
        Ok(Response::ok())
    }
}
