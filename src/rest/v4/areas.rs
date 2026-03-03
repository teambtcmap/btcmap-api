use crate::db::main::MainPool;
use crate::rest::error::RestResult as Res;
use crate::rest::error::{RestApiError, RestApiErrorCode};
use crate::service;
use actix_web::{get, web::Data, web::Json, web::Query};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SearchArgs {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize, Deserialize)]
pub struct AreaSearchResult {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub url_alias: String,
    pub icon: Option<String>,
    pub website_url: String,
}

#[get("")]
pub async fn get(args: Query<SearchArgs>, pool: Data<MainPool>) -> Res<Vec<AreaSearchResult>> {
    let lat = args.lat;
    let lon = args.lon;

    if !(-90.0..=90.0).contains(&lat) {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Latitude must be between -90 and 90",
        ));
    }

    if !(-180.0..=180.0).contains(&lon) {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Longitude must be between -180 and 180",
        ));
    }

    let areas = service::area::find_areas_by_lat_lon(lat, lon, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let results: Vec<AreaSearchResult> = areas
        .into_iter()
        .map(|area| {
            let r#type = area.tags.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let singular_type = if let Some(stripped) = r#type.strip_suffix("ies") {
                format!("{}y", stripped)
            } else if let Some(stripped) = r#type.strip_suffix('s') {
                stripped.to_string()
            } else {
                r#type.to_string()
            };
            let url_alias = area.alias();
            AreaSearchResult {
                id: area.id,
                name: area.name(),
                r#type: r#type.to_string(),
                url_alias: url_alias.clone(),
                icon: area
                    .tags
                    .get("icon:square")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                website_url: format!(
                    "https://btcmap.org/{}/{}",
                    singular_type, url_alias
                ),
            }
        })
        .collect();

    Ok(Json(results))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::area::schema::Area;
    use crate::db::main::test::pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::json;

    #[test]
    async fn search_invalid_lat_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?lat=91&lon=0").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_invalid_lon_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?lat=0&lon=181").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_returns_results() -> Result<()> {
        let pool = pool();

        let mut tags = Area::mock_tags();
        tags.insert("name".into(), json!("Phuket"));
        tags.insert("type".into(), json!("country"));
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [98.2181205776469, 8.20412838698085],
                              [98.2181205776469, 7.74024270965898],
                              [98.4806081271079, 7.74024270965898],
                              [98.4806081271079, 8.20412838698085],
                              [98.2181205776469, 8.20412838698085]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        db::main::area::queries::insert(tags, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?lat=7.9&lon=98.3").to_request();
        let res: Vec<AreaSearchResult> = test::call_and_read_body_json(&app, req).await;

        assert!(!res.is_empty());
        assert_eq!(res[0].name, "Phuket");
        Ok(())
    }
}
