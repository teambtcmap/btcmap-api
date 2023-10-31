use std::collections::HashMap;

use crate::model::Element;
use crate::service::auth::get_admin_token;
use crate::service::overpass::OverpassElement;
use crate::ApiError;
use actix_web::get;
use actix_web::patch;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i32>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: String,
    pub osm_json: OverpassElement,
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Element {
    fn into(self) -> GetItem {
        GetItem {
            id: self.overpass_data.btcmap_id(),
            osm_json: self.overpass_data,
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

impl Into<Json<GetItem>> for Element {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
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

#[get("{id}")]
pub async fn get_by_osm_type_and_id(
    id: Path<String>,
    conn: Data<Connection>,
) -> Result<Json<GetItem>, ApiError> {
    let id_parts: Vec<&str> = id.split(":").collect();
    let r#type = id_parts[0];
    let id = id_parts[1].parse::<i64>()?;
    Element::select_by_osm_type_and_id(r#type, id, &conn)?
        .map(|it| it.into())
        .ok_or(ApiError::new(
            404,
            &format!("Element with id {id} doesn't exist"),
        ))
}

#[patch("{id}/tags")]
async fn patch_tags(
    args: Json<HashMap<String, Value>>,
    conn: Data<Connection>,
    id: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let id_parts: Vec<&str> = id.split(":").collect();
    let r#type = id_parts[0];
    let id = id_parts[1].parse::<i64>()?;
    warn!(
        token.user_id,
        r#type, id, "User attempted to merge new tags",
    );
    match Element::select_by_osm_type_and_id(r#type, id, &conn)? {
        Some(element) => element.patch_tags(&args, &conn)?,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no element with type = {type} and id = {id}"),
            ));
        }
    };
    Ok(HttpResponse::Ok())
}

#[post("{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let id_parts: Vec<&str> = id.split(":").collect();
    let r#type = id_parts[0];
    let id = id_parts[1].parse::<i64>()?;
    warn!(
        deprecated_api = true,
        user_id = token.user_id,
        id,
        tag_name = args.name,
        tag_value = args.value,
        "User attempted to update element tag",
    );
    let element = Element::select_by_osm_type_and_id(r#type, id, &conn)?;
    match element {
        Some(element) => {
            if args.value.len() > 0 {
                element.insert_tag(&args.name, &args.value, &conn)?;
            } else {
                element.delete_tag(&args.name, &conn)?;
            }
            Ok(HttpResponse::Created())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no element with id {type}:{id}"),
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::db;
    use crate::model::token;
    use crate::test::mock_conn;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use rusqlite::named_params;
    use serde_json::json;
    use time::macros::datetime;

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);

        Ok(())
    }

    #[actix_web::test]
    async fn get_one_row() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
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
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?;
        Element::insert(&OverpassElement::mock(2), &conn)?;
        Element::insert(&OverpassElement::mock(3), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);

        Ok(())
    }

    #[actix_web::test]
    async fn get_updated_since() -> Result<()> {
        let conn = mock_conn();
        Element::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&datetime!(2022-01-05 00:00 UTC), &conn)?;
        Element::insert(&OverpassElement::mock(2), &conn)?
            .set_updated_at(&datetime!(2022-02-05 00:00 UTC), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }

    #[actix_web::test]
    async fn get_by_osm_type_and_id() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::get_by_osm_type_and_id),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}", element.id))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, element.into());
        Ok(())
    }

    #[actix_web::test]
    async fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{}/tags", element.id))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[actix_web::test]
    async fn post_tags() -> Result<()> {
        let conn = mock_conn();
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::post_tags),
        )
        .await;
        let req = TestRequest::post()
            .uri(&format!("/{}/tags", element.id))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_form(PostTagsArgs {
                name: "foo".into(),
                value: "bar".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        Ok(())
    }
}
