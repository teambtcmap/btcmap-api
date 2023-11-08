use crate::report::model::ReportRepo;
use crate::service::AuthService;
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
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::warn;

use super::Report;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Report {
    fn into(self) -> GetItem {
        let area_id = if self.area_url_alias == "earth" {
            "".into()
        } else {
            self.area_url_alias
        };

        GetItem {
            id: self.id,
            area_id,
            date: self.date.to_string(),
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

impl Into<Json<GetItem>> for Report {
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
async fn get(args: Query<GetArgs>, repo: Data<ReportRepo>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => repo
            .select_updated_since(updated_since, args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => repo
            .select_all(args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, repo: Data<ReportRepo>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();
    repo.select_by_id(id)
        .await?
        .map(|it| it.into())
        .ok_or(ApiError::new(
            404,
            &format!("Report with id = {id} doesn't exist"),
        ))
}

#[patch("{id}/tags")]
async fn patch_tags(
    req: HttpRequest,
    id: Path<i64>,
    args: Json<HashMap<String, Value>>,
    auth: Data<AuthService>,
    repo: Data<ReportRepo>,
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;
    let report_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    warn!(
        user_id = token.user_id,
        report_id,
        tags = keys.join(", "),
        "User attempted to update report tags",
    );

    repo.select_by_id(report_id).await?.ok_or(ApiError::new(
        404,
        &format!("Report with id = {report_id} doesn't exist"),
    ))?;

    repo.patch_tags(report_id, &args).await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::token;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use rusqlite::named_params;
    use serde_json::{json, Value};

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state();
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state();
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state.conn.execute(
            "INSERT INTO report (
                        area_id,
                        date,
                        updated_at
                    ) VALUES (
                        1,
                        '2023-05-06',
                        '2023-05-06T00:00:00Z'
                    )",
            [],
        )?;
        state.conn.execute(
            "INSERT INTO report (
                        area_id,
                        date,
                        updated_at
                    ) VALUES (
                        1,
                        '2023-05-07',
                        '2023-05-07T00:00:00Z'
                    )",
            [],
        )?;
        state.conn.execute(
            "INSERT INTO report (
                        area_id,
                        date,
                        updated_at
                    ) VALUES (
                        1,
                        '2023-05-08',
                        '2023-05-08T00:00:00Z'
                    )",
            [],
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state();
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state.conn.execute(
                    "INSERT INTO report (area_id, date, updated_at) VALUES (1, '2022-01-05', '2022-01-05T00:00:00Z')",
                    [],
                )?;
        state.conn.execute(
                    "INSERT INTO report (area_id, date, updated_at) VALUES (1, '2022-02-05', '2022-02-05T00:00:00Z')",
                    [],
                )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state();
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        let admin_token = "test";
        state.conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        state.conn.execute(
                    "INSERT INTO report (area_id, date, updated_at) VALUES (1, '2020-01-01', '2022-01-05T00:00:00Z')",
                    [],
                )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.report_repo))
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
