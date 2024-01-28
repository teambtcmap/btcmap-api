use crate::{auth::AuthService, discord, user::UserRepo, Error};
use actix_web::{
    patch,
    web::{Data, Json, Path},
    HttpRequest, HttpResponse, Responder,
};
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;

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
    let log_message = format!(
        "User {} patched tags for user https://api.btcmap.org/v2/users/{} {}",
        token.owner,
        id,
        serde_json::to_string_pretty(&args).unwrap(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
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
        let user = state.user_repo.insert(1, &OsmUser::mock()).await?;
        let token = state.auth.mock_token("test").await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.user_repo))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{}/tags", user.id))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }
}
