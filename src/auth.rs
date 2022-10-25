use crate::model::ApiError;
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
