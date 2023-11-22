use super::Report;
use crate::report::model::ReportRepo;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::Duration;
use time::OffsetDateTime;

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

#[get("")]
async fn get(args: Query<GetArgs>, repo: Data<ReportRepo>) -> Result<Json<Vec<GetItem>>, Error> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => repo
            .select_updated_since(updated_since, args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => repo
            .select_updated_since(
                &OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(7))
                    .unwrap()
                    .format(&Rfc3339)
                    .unwrap(),
                args.limit,
            )
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, repo: Data<ReportRepo>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    repo.select_by_id(id)
        .await?
        .map(|it| it.into())
        .ok_or(Error::HttpNotFound(format!(
            "Report with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::report::v2::GetItem;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::{Map, Value};
    use std::collections::HashMap;
    use time::macros::{date, datetime};
    use time::OffsetDateTime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state
            .report_repo
            .insert(1, &date!(2023 - 05 - 06), &HashMap::new())
            .await?;
        state
            .report_repo
            .insert(1, &date!(2023 - 05 - 07), &HashMap::new())
            .await?;
        state
            .report_repo
            .insert(1, &date!(2023 - 05 - 08), &HashMap::new())
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
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
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        let report_1 = state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .set_updated_at(report_1.id, &datetime!(2022-01-05 00:00:00 UTC))
            .await?;
        let report_2 = state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .set_updated_at(report_2.id, &datetime!(2022-02-05 00:00:00 UTC))
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.report_repo))
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
}
