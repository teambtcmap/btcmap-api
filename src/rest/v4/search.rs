use crate::db;
use crate::rest::error::RestResult as Res;
use crate::rest::error::{RestApiError, RestApiErrorCode};
use actix_web::{get, web::Data, web::Json, web::Query};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SearchArgs {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub type_filter: Option<String>, // "area", "element", or none
}

fn default_limit() -> i64 {
    20
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub name: String,
    pub r#type: String,
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: u32,
    pub has_more: bool,
    pub query: String,
    pub pagination: PaginationInfo,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaginationInfo {
    pub offset: i64,
    pub limit: i64,
    pub total: u32,
}

// GET /search?q=query&limit=10&offset=0&type_filter=area
#[get("")]
pub async fn get(args: Query<SearchArgs>, pool: Data<Pool>) -> Res<SearchResponse> {
    // query validation
    let query = args.q.trim();
    if query.is_empty() {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Search query cannot be empty",
        ));
    }

    // minimal query length
    if query.len() < 2 {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Search query must be at least 2 characters long",
        ));
    }

    // maximal length
    let limit = args.limit.min(100);
    let offset = args.offset.max(0);

    let mut results = Vec::new();

    // search areas by default or if specified
    if args.type_filter.is_none() || args.type_filter.as_ref().map(|s| s.as_str()) == Some("area") {
        let areas = db::area::queries::select_by_search_query(query, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

        for area in areas {
            results.push(SearchResult {
                name: area.name(),
                r#type: "area".to_string(),
                id: area.id,
            });
        }
    }

    // search elements by default or if specified
    if args.type_filter.is_none()
        || args.type_filter.as_ref().map(|s| s.as_str()) == Some("element")
    {
        let elements = db::element::queries::select_by_search_query(query, false, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

        for element in elements {
            results.push(SearchResult {
                name: element.name(),
                r#type: "element".to_string(),
                id: element.id,
            });
        }
    }

    // relevance sort
    results.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase().starts_with(&query.to_lowercase());
        let b_exact = b.name.to_lowercase().starts_with(&query.to_lowercase());
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    // pagination
    let total = results.len() as u32;
    let start = offset as usize;
    let end = (start + limit as usize).min(results.len());
    let paginated_results = if start < results.len() {
        results[start..end].to_vec()
    } else {
        Vec::new()
    };

    let response = SearchResponse {
        results: paginated_results,
        total_count: total,
        has_more: (offset + limit) < total as i64,
        query: query.to_string(),
        pagination: PaginationInfo {
            offset,
            limit,
            total,
        },
    };

    Ok(Json(response))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};

    #[test]
    async fn search_empty_query_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/search").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/search?q=").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_too_short_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/search").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/search?q=a").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_valid_query_returns_results() -> Result<()> {
        let pool = pool();
        let _element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/search").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/search?q=cuba").to_request();
        let res: SearchResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.query, "cuba");
        assert!(res.results.is_empty());
        Ok(())
    }

    #[test]
    async fn search_with_pagination_works() -> Result<()> {
        let pool = pool();
        for i in 1..=5 {
            let _element = db::element::queries::insert(OverpassElement::mock(i), &pool).await?;
        }
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/search").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/search?q=test&limit=2&offset=0")
            .to_request();
        let res: SearchResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.pagination.limit, 2);
        assert_eq!(res.pagination.offset, 0);
        assert!(res.results.len() <= 2);
        Ok(())
    }

    #[test]
    async fn search_with_type_filter_element_only() -> Result<()> {
        let pool = pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/search").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/search?q=test&type_filter=element")
            .to_request();
        let res: SearchResponse = test::call_and_read_body_json(&app, req).await;
        for result in res.results {
            assert_eq!(result.r#type, "element");
        }
        Ok(())
    }
}
