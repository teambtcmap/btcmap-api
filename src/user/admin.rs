use crate::{auth::AuthService, user::UserRepo, ApiError};
use actix_web::{
    patch,
    web::{Data, Json, Path},
    HttpRequest, HttpResponse, Responder,
};
use http::StatusCode;
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
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;
    let user_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    warn!(
        actor_id = token.user_id,
        user_id,
        tags = keys.join(", "),
        "User attempted to update user tags",
    );

    repo.select_by_id(user_id).await?.ok_or(ApiError::new(
        StatusCode::NOT_FOUND,
        &format!("User with id = {user_id} doesn't exist"),
    ))?;

    repo.patch_tags(user_id, &args).await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use crate::auth::Token;
    use crate::osm::osm::OsmUser;
    use crate::test::mock_state;
    use crate::user::User;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let user_id = 1;
        User::insert(user_id, &OsmUser::mock(), &state.conn)?;
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
