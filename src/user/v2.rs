use crate::firewall;
use crate::osm::osm::OsmUser;
use crate::user::User;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::web::Redirect;
use actix_web::Either;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub osm_json: OsmUser,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for User {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            osm_json: self.osm_data,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self
                .deleted_at
                .map(|it| it.format(&Rfc3339).unwrap())
                .unwrap_or_default()
                .into(),
        }
    }
}

impl Into<Json<GetItem>> for User {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    let started_at = Instant::now();
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/users.json").permanent(),
        ));
    }
    let users = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => User::select_updated_since(updated_since, args.limit, conn),
            None => User::select_all(args.limit, conn),
        })
        .await??;
    let users_len = users.len() as i64;
    let res = Either::Left(Json(users.into_iter().map(|it| it.into()).collect()));
    let time_ms = Instant::now().duration_since(started_at).as_millis() as i64;
    firewall::log_sync_api_request(&req, "v2/users", users_len, time_ms)?;
    Ok(res)
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Arc<Pool>>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| User::select_by_id(id, conn))
        .await??
        .map(|it| it.into())
        .ok_or(Error::HttpNotFound(format!(
            "User with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::osm::osm::OsmUser;
    use crate::test::mock_state;
    use crate::user::v2::GetItem;
    use crate::user::User;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Value;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state().await;
        User::insert(1, &OsmUser::mock(), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=100").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state().await;
        state.pool.get().await?.interact(|conn| {
            conn.execute(
                "INSERT INTO user (rowid, osm_data, updated_at) VALUES (1, json(?), '2022-01-05T00:00:00Z')",
                [serde_json::to_string(&OsmUser::mock()).unwrap()],
            ).unwrap();
            conn.execute(
                "INSERT INTO user (rowid, osm_data, updated_at) VALUES (2, json(?), '2022-02-05T00:00:00Z')",
                [serde_json::to_string(&OsmUser::mock()).unwrap()],
            ).unwrap();
        }).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let state = mock_state().await;
        let user_id = 1;
        User::insert(user_id, &OsmUser::mock(), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{user_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, user_id);
        Ok(())
    }
}
