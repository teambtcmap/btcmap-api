use crate::area::Area;
use crate::area::AreaRepo;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::web::Redirect;
use actix_web::Either;
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
    pub id: String,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Area {
    fn into(self) -> GetItem {
        GetItem {
            id: self.tags["url_alias"].as_str().unwrap().into(),
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

impl Into<Json<GetItem>> for Area {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
async fn get(
    args: Query<GetArgs>,
    repo: Data<AreaRepo>,
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/areas.json").permanent(),
        ));
    }

    Ok(Either::Left(Json(match &args.updated_since {
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
    })))
}

#[get("{url_alias}")]
async fn get_by_url_alias(
    url_alias: Path<String>,
    repo: Data<AreaRepo>,
) -> Result<Json<GetItem>, Error> {
    repo.select_by_url_alias(&url_alias)
        .await?
        .ok_or(Error::HttpNotFound(format!(
            "Area with url_alias = {url_alias} doesn't exist"
        )))
        .map(|it| it.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state().await;
        let mut tags = Map::new();
        tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
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
        let state = mock_state().await;
        let mut tags = Map::new();
        tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&tags).await?;
        state.area_repo.insert(&tags).await?;
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let state = mock_state().await;
        let area_url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(area_url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::get_by_url_alias),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{area_url_alias}"))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, area_url_alias);
        Ok(())
    }
}
