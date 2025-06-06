use super::Event;
use crate::log::RequestExtension;
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
use serde_json::Value;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::Duration;
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
    pub user_id: i64,
    pub element_id: String,
    pub r#type: String,
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl From<Event> for GetItem {
    fn from(val: Event) -> GetItem {
        GetItem {
            id: val.id,
            user_id: val.user_id,
            element_id: format!("{}:{}", val.element_osm_type, val.element_osm_id),
            r#type: val.r#type,
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

impl From<Event> for Json<GetItem> {
    fn from(val: Event) -> Json<GetItem> {
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
            Redirect::to("https://static.btcmap.org/api/v2/events.json").permanent(),
        ));
    }
    let events = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => Event::select_updated_since(updated_since, args.limit, conn),
            None => Event::select_updated_since(
                &OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(30))
                    .unwrap(),
                args.limit,
                conn,
            ),
        })
        .await??;
    let events_len = events.len();
    let res = Either::Left(Json(events.into_iter().map(|it| it.into()).collect()));
    req.extensions_mut()
        .insert(RequestExtension::new(events_len));
    Ok(res)
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| Event::select_by_id(id, conn))
        .await??
        .map(|it| it.into())
        .ok_or(Error::not_found())
}

#[cfg(test)]
mod test {
    use crate::event::v2::GetItem;
    use crate::event::Event;
    use crate::osm::api::EditingApiUser;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_db;
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
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &db.conn)?;
        Event::insert(user.id, element.id, "", &db.conn)?;
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
    async fn get_with_limit() -> Result<()> {
        let db = mock_db();
        db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        db::element::queries::insert(&OverpassElement::mock(1), &db.conn)?;
        Event::insert(1, 1, "", &db.conn)?;
        Event::insert(1, 1, "", &db.conn)?;
        Event::insert(1, 1, "", &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let db = mock_db();
        db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        db::element::queries::insert(&OverpassElement::mock(1), &db.conn)?;
        let event_1 = Event::insert(1, 1, "", &db.conn)?;
        Event::set_updated_at(event_1.id, &datetime!(2022-01-05 00:00:00 UTC), &db.conn)?;
        let event_2 = Event::insert(1, 1, "", &db.conn)?;
        Event::set_updated_at(event_2.id, &datetime!(2022-02-05 00:00:00 UTC), &db.conn)?;
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
        let event_id = 1;
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &db.conn)?;
        Event::insert(user.id, element.id, "", &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{event_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, event_id);
        Ok(())
    }
}
