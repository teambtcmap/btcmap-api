use crate::{area::AreaRepo, auth::AuthService, ApiError};
use actix_web::{
    post,
    web::{Data, Json},
    HttpRequest, Responder,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    tags: HashMap<String, Value>,
}

#[post("")]
async fn post(
    req: HttpRequest,
    args: Json<PostArgs>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    auth.check(&req).await?;
    if !args.tags.contains_key("url_alias") {
        Err(ApiError::new(500, format!("url_alias is missing")))?
    }
    let url_alias = &args.tags.get("url_alias").unwrap();
    if !url_alias.is_string() {
        Err(ApiError::new(500, format!("url_alias should be a string")))?
    }
    let url_alias = url_alias.as_str().unwrap();
    if let Some(_) = repo.select_by_url_alias(url_alias).await? {
        Err(ApiError::new(
            303,
            format!("Area with url_alias = {} already exists", url_alias),
        ))?
    }
    repo.insert(&args.tags).await.map_err(|_| {
        ApiError::new(
            500,
            format!("Failed to insert area with url_alias = {}", url_alias),
        )
    })?;
    Ok(Json(json!({
        "message": format!("Area with url_alias = {} has been created", url_alias),
    })))
}

#[cfg(test)]
mod tests {
    use crate::area::AreaRepo;
    use crate::auth::Token;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Value;

    #[test]
    async fn post() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::post)),
        )
        .await;
        let args = r#"
        {
            "tags": {
                "url_alias": "test-area",
                "string": "bar",
                "int": 5,
                "float": 12.34,
                "bool": false
            }
        }
        "#;
        let args: Value = serde_json::from_str(args)?;
        let req = TestRequest::post()
            .uri("/")
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_success());
        let area = state
            .area_repo
            .select_by_url_alias("test-area")
            .await?
            .unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["int"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }
}
