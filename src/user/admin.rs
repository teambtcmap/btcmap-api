use crate::{
    auth::{self},
    discord,
    user::UserRepo,
    Error,
};
use actix_web::{
    patch,
    web::{Data, Json, Path},
    HttpRequest, HttpResponse, Responder,
};
use deadpool_sqlite::Pool;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

#[patch("{id}/tags")]
pub async fn patch_tags(
    req: HttpRequest,
    id: Path<i64>,
    args: Json<HashMap<String, Value>>,
    pool: Data<Arc<Pool>>,
    repo: Data<UserRepo>,
) -> Result<impl Responder, Error> {
    let token = auth::service::check(&req, &pool).await?;
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
    use crate::{auth, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use serde_json::json;

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state().await;
        let user = state.user_repo.insert(1, &OsmUser::mock()).await?;
        let token = auth::service::mock_token("test", &state.pool).await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
