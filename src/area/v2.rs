use crate::area::Area;
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
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/areas.json").permanent(),
        ));
    }
    let areas = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => Area::select_updated_since(updated_since, args.limit, conn),
            None => Area::select_all(conn),
        })
        .await??;
    let areas_len = areas.len() as i64;
    let res = Either::Left(Json(areas.into_iter().map(|it| it.into()).collect()));
    req.extensions_mut()
        .insert(RequestExtension::new("v2/areas", areas_len));
    Ok(res)
}

#[get("{url_alias}")]
pub async fn get_by_url_alias(
    url_alias: Path<String>,
    pool: Data<Pool>,
) -> Result<Json<GetItem>, Error> {
    let cloned_url_alias = url_alias.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_alias(&cloned_url_alias, conn))
        .await??;
    area.ok_or(Error::NotFound(format!(
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
    use geojson::{Feature, GeoJson};

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        Area::insert(
            GeoJson::Feature(Feature::default()),
            tags,
            "test",
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        Area::insert(
            GeoJson::Feature(Feature::default()),
            tags.clone(),
            "test",
            &state.conn,
        )?;
        Area::insert(
            GeoJson::Feature(Feature::default()),
            tags.clone(),
            "test",
            &state.conn,
        )?;
        Area::insert(
            GeoJson::Feature(Feature::default()),
            tags,
            "test",
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
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
        Area::insert(
            GeoJson::Feature(Feature::default()),
            tags,
            "test",
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
