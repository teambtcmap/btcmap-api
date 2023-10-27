use std::collections::HashMap;

use crate::model::Element;
use crate::service::overpass::OverpassElement;
use crate::ApiError;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i32>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_data: Option<OverpassElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for Element {
    fn into(self) -> GetItem {
        let osm_data = if self.deleted_at.is_none() {
            Some(self.overpass_json)
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
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for Element {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(
    args: Query<GetArgs>,
    conn: Data<Connection>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => Element::select_updated_since(&updated_since, args.limit, &conn)?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => Element::select_all(args.limit, &conn)?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::db;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn get_empty_array() -> Result<()> {
        let conn = db::setup_connection()?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[actix_web::test]
    async fn get_not_empty_array() -> Result<()> {
        let conn = db::setup_connection()?;
        let element = Element::insert(&OverpassElement::mock(), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], element.into());
        Ok(())
    }

    #[actix_web::test]
    async fn get_with_limit() -> Result<()> {
        let conn = db::setup_connection()?;
        let element_1 = Element::insert(
            &OverpassElement {
                r#type: "node".into(),
                id: 1,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        let element_2 = Element::insert(
            &OverpassElement {
                r#type: "node".into(),
                id: 2,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        Element::insert(
            &OverpassElement {
                r#type: "node".into(),
                id: 3,
                ..OverpassElement::mock()
            },
            &conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], element_1.into());
        assert_eq!(res[1], element_2.into());
        Ok(())
    }
}
