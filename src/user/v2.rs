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
use actix_web::web::Redirect;
use actix_web::Either;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
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
    pub osm_json: EditingApiUser,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl From<OsmUser> for GetItem {
    fn from(val: OsmUser) -> Self {
        GetItem {
            id: val.id,
            osm_json: val.osm_data,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val
                .deleted_at
                .map(|it| it.format(&Rfc3339).unwrap())
                .unwrap_or_default(),
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
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/users.json").permanent(),
        ));
    }
    let users = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => {
                db::osm_user::queries::select_updated_since(updated_since, args.limit, conn)
            }
            None => db::osm_user::queries::select_all(args.limit, conn),
        })
        .await??;
    let users_len = users.len();
    let res = Either::Left(Json(users.into_iter().map(|it| it.into()).collect()));
    req.extensions_mut()
        .insert(RequestExtension::new(users_len));
    Ok(res)
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
    use crate::user::v2::GetItem;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Value;
    use time::macros::datetime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let db = mock_db();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        db.pool
            .get()
            .await?
            .interact(|conn| {
                let _u1 = db::osm_user::queries::insert(1, &EditingApiUser::mock(), conn).unwrap();
                db::osm_user::queries::set_updated_at(
                    _u1.id,
                    &datetime!(2022-01-05 00:00:00 UTC),
                    conn,
                )
                .unwrap();
                let _u2 = db::osm_user::queries::insert(2, &EditingApiUser::mock(), conn).unwrap();
                db::osm_user::queries::set_updated_at(
                    _u2.id,
                    &datetime!(2022-02-05 00:00:00 UTC),
                    conn,
                )
                .unwrap();
            })
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db();
        let user_id = 1;
        db::osm_user::queries::insert(user_id, &EditingApiUser::mock(), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{user_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, user_id);
        Ok(())
    }
}
