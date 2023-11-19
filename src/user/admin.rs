use crate::{auth::AuthService, user::UserRepo, Error};
use actix_web::{
    patch,
    web::{Data, Json, Path},
    HttpRequest, HttpResponse, Responder,
};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

#[patch("{id}/tags")]
async fn patch_tags(
    req: HttpRequest,
    id: Path<i64>,
    args: Json<HashMap<String, Value>>,
    auth: Data<AuthService>,
    repo: Data<UserRepo>,
) -> Result<impl Responder, Error> {
    let token = auth.check(&req).await?;
    repo.select_by_id(*id)
        .await?
        .ok_or(Error::HttpNotFound(format!(
            "User with id = {id} doesn't exist"
        )))?;
    repo.patch_tags(*id, &args).await?;
    debug!(
        admin_channel_message = format!(
            "User https://api.btcmap.org/v2/users/{} patched tags for user https://api.btcmap.org/v2/users/{} {}",
            token.user_id, id, serde_json::to_string_pretty(&args).unwrap(),
        )
    );
    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use crate::osm::osm::OsmUser;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state().await;
        let user_id = 1;
        state.user_repo.insert(user_id, &OsmUser::mock()).await?;
        let token = state.auth.mock_token(1, "test").await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.user_repo))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{user_id}/tags"))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }
}
