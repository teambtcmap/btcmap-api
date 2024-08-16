use crate::osm::osm::OsmUser;
use crate::user::User;
use crate::user::UserRepo;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    limit: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_data: Option<OsmUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for User {
    fn into(self) -> GetItem {
        let osm_data = if self.deleted_at.is_none() {
            Some(self.osm_data)
        } else {
            None
        };
        let tags = if self.deleted_at.is_none() {
            Some(self.tags)
        } else {
            None
        };
        GetItem {
            id: self.id,
            osm_data,
            tags,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for User {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(args: Query<GetArgs>, repo: Data<UserRepo>) -> Result<Json<Vec<GetItem>>, Error> {
    Ok(Json(
        repo.select_updated_since(&args.updated_since, Some(args.limit))
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    ))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, repo: Data<UserRepo>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    repo.select_by_id(id)
        .await?
        .map(|it| it.into())
        .ok_or(Error::HttpNotFound(format!(
            "User with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::error::{self, ApiError};
    use crate::osm::osm::OsmUser;
    use crate::test::mock_state;
    use crate::user::UserRepo;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data, QueryConfig};
    use actix_web::{test, App};
    use http::StatusCode;
    use time::macros::datetime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(Data::new(UserRepo::mock()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: ApiError = test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `updated_since`"));
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(Data::new(UserRepo::mock()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z")
            .to_request();
        let res: ApiError = test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `limit`"));
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.user_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let state = mock_state().await;
        let user = state.user_repo.insert(1, &OsmUser::mock()).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.user_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![user.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state().await;
        let user_1 = state.user_repo.insert(1, &OsmUser::mock()).await?;
        let user_2 = state.user_repo.insert(2, &OsmUser::mock()).await?;
        let _user_3 = state.user_repo.insert(3, &OsmUser::mock()).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.user_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![user_1.into(), user_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state().await;
        let user_1 = state.user_repo.insert(1, &OsmUser::mock()).await?;
        state
            .user_repo
            .set_updated_at(user_1.id, &datetime!(2022-01-05 00:00 UTC))
            .await?;
        let user_2 = state.user_repo.insert(2, &OsmUser::mock()).await?;
        let user_2 = state
            .user_repo
            .set_updated_at(user_2.id, &datetime!(2022-02-05 00:00 UTC))
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.user_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z&limit=100")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![user_2.into()]);
        Ok(())
    }
}
