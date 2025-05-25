use super::Event;
use crate::log::RequestExtension;
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
use serde_json::Value;
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, Value>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Event> for GetItem {
    fn from(val: Event) -> GetItem {
        let user_id = if val.deleted_at.is_none() {
            Some(val.user_id)
        } else {
            None
        };
        let element_id = if val.deleted_at.is_none() {
            Some(val.element_id)
        } else {
            None
        };
        let r#type = if val.deleted_at.is_none() {
            Some(match val.r#type.as_str() {
                "create" => 1,
                "update" => 2,
                "delete" => 3,
                _ => -1,
            })
        } else {
            None
        };
        let tags = if val.deleted_at.is_none() && !val.tags.is_empty() {
            Some(val.tags)
        } else {
            None
        };
        let created_at = if val.deleted_at.is_none() {
            Some(val.created_at)
        } else {
            None
        };
        GetItem {
            id: val.id,
            user_id,
            element_id,
            r#type,
            tags,
            created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<Event> for Json<GetItem> {
    fn from(val: Event) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let events = pool
        .get()
        .await?
        .interact(move |conn| match args.updated_since {
            Some(updated_since) => {
                Event::select_updated_since(&updated_since, Some(args.limit.unwrap_or(100)), conn)
            }
            None => Event::select_all(Some("DESC".into()), Some(args.limit.unwrap_or(100)), conn),
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(events.len()));
    Ok(Json(events.into_iter().map(|it| it.into()).collect()))
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
    use crate::element::Element;
    use crate::event::Event;
    use crate::osm::api::EditingApiUser;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_db;
    use crate::user::OsmUser;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use time::macros::datetime;

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
        let user = OsmUser::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let event = Event::insert(user.id, element.id, "", &db.conn)?;
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
        assert_eq!(res, vec![event.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let db = mock_db();
        let user = OsmUser::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let event_1 = Event::insert(user.id, element.id, "", &db.conn)?;
        let event_2 = Event::insert(user.id, element.id, "", &db.conn)?;
        let _event_3 = Event::insert(user.id, element.id, "", &db.conn)?;
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
        assert_eq!(res, vec![event_1.into(), event_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let db = mock_db();
        let user = OsmUser::insert(1, &EditingApiUser::mock(), &db.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let event_1 = Event::insert(user.id, element.id, "", &db.conn)?;
        Event::set_updated_at(event_1.id, &datetime!(2022-01-05 00:00 UTC), &db.conn)?;
        let event_2 = Event::insert(user.id, element.id, "", &db.conn)?;
        let event_2 =
            Event::set_updated_at(event_2.id, &datetime!(2022-02-05 00:00 UTC), &db.conn)?;
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
        assert_eq!(res, vec![event_2.into()]);
        Ok(())
    }
}
