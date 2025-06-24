use crate::db;
use crate::db::osm_user::queries::OsmUser;
use crate::log::RequestExtension;
use crate::osm::api::EditingApiUser;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
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
    pub osm_data: Option<EditingApiUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<OsmUser> for GetItem {
    fn from(val: OsmUser) -> Self {
        let osm_data = if val.deleted_at.is_none() {
            Some(val.osm_data)
        } else {
            None
        };
        let tags = if val.deleted_at.is_none() {
            Some(val.tags)
        } else {
            None
        };
        GetItem {
            id: val.id,
            osm_data,
            tags,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<OsmUser> for Json<GetItem> {
    fn from(val: OsmUser) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let users = pool
        .get()
        .await?
        .interact(move |conn| {
            db::osm_user::queries::select_updated_since(&args.updated_since, Some(args.limit), conn)
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(users.len()));
    Ok(Json(users.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| db::osm_user::queries::select_by_id(id, conn))
        .await?
        .map(|it| it.into())
}

#[cfg(test)]
mod test {
    use crate::osm::api::EditingApiUser;
    use crate::test::mock_db;
    use crate::{db, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use time::macros::datetime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(mock_db().pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(mock_db().pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let db = mock_db();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        let user_1 = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let user_2 = db::osm_user::queries::insert(2, &EditingApiUser::mock(), &db.conn)?;
        let _user_3 = db::osm_user::queries::insert(3, &EditingApiUser::mock(), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        let user_1 = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        db::osm_user::queries::set_updated_at(
            user_1.id,
            &datetime!(2022-01-05 00:00 UTC),
            &db.conn,
        )?;
        let user_2 = db::osm_user::queries::insert(2, &EditingApiUser::mock(), &db.conn)?;
        let user_2 = db::osm_user::queries::set_updated_at(
            user_2.id,
            &datetime!(2022-02-05 00:00 UTC),
            &db.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
