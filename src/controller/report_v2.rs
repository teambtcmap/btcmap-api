use crate::model::report;
use crate::model::Report;
use crate::service::auth::get_admin_token;
use crate::ApiError;
use actix_web::get;
use actix_web::patch;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use tracing::warn;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for Report {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            area_id: self.area_id,
            date: self.date,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[get("")]
async fn get(args: Query<GetArgs>, db: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(report::SELECT_UPDATED_SINCE)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since,
                    ":limit": args.limit.unwrap_or(std::i32::MAX)
                },
                report::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(report::SELECT_ALL)?
            .query_map(
                named_params! { ":limit": args.limit.unwrap_or(std::i32::MAX) },
                report::SELECT_ALL_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        report::SELECT_BY_ID,
        &[(":id", &id)],
        report::SELECT_BY_ID_MAPPER,
    )
    .optional()?
    .map(|it| Json(it.into()))
    .ok_or(ApiError::new(
        404,
        &format!("Report with id {id} doesn't exist"),
    ))
}

#[patch("{id}/tags")]
async fn patch_tags(
    args: Json<Map<String, Value>>,
    db: Data<Connection>,
    id: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let report_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    warn!(
        user_id = token.user_id,
        report_id,
        tags = keys.join(", "),
        "User attempted to update report tags",
    );

    let report: Option<Report> = db
        .query_row(
            report::SELECT_BY_ID,
            named_params! { ":id": report_id },
            report::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    let report = match report {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no report with id {report_id}"),
            ));
        }
    };

    let mut old_tags = report.tags.clone();

    let mut merged_tags = Map::new();
    merged_tags.append(&mut old_tags);
    merged_tags.append(&mut args.clone());

    db.execute(
        report::UPDATE_TAGS,
        named_params! {
            ":report_id": report_id,
            ":tags": serde_json::to_string(&merged_tags).unwrap(),
        },
    )?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db::tests::db;
    use crate::model::token;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use rusqlite::named_params;
    use serde_json::{json, Value};

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db()?))
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
        let db = db()?;
        db.execute(
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":tags" : "{}",
            },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[actix_web::test]
    async fn get_with_limit() -> Result<()> {
        let db = db()?;
        db.execute(
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":tags" : "{}",
            },
        )?;
        db.execute(
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":tags" : "{}",
            },
        )?;
        db.execute(
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":tags" : "{}",
            },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[actix_web::test]
    async fn get_updated_since() -> Result<()> {
        let db = db()?;
        db.execute(
            "INSERT INTO report (area_id, date, updated_at) VALUES ('', '', '2022-01-05')",
            [],
        )?;
        db.execute(
            "INSERT INTO report (area_id, date, updated_at) VALUES ('', '', '2022-02-05')",
            [],
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
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
    async fn patch_tags() -> Result<()> {
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        db.execute(
            "INSERT INTO report (area_id, date, updated_at) VALUES ('', '', '2022-01-05')",
            [],
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/1/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }
}
