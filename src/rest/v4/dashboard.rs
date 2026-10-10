use crate::db::log::LogPool;
use crate::db::main::user::schema::Role;
use crate::rest::auth::Auth;
use crate::rpc::analytics::dashboard as analytics_dashboard;
use crate::{
    db,
    db::main::MainPool,
    rest::error::{RestApiError, RestResult},
};
use actix_web::{
    get,
    web::{Data, Json},
};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

#[derive(Serialize, ts_rs::TS)]
#[ts(export)]
pub struct Dashboard {
    #[ts(type = "number")]
    pub total_merchants: i64,
    pub total_merchants_chart: Vec<ChartEntry>,
    #[ts(type = "number")]
    pub verified_merchants_1y: i64,
    pub verified_merchants_1y_chart: Vec<ChartEntry>,
    #[ts(type = "number")]
    pub total_exchanges: i64,
    pub total_exchanges_chart: Vec<ChartEntry>,
    #[ts(type = "number")]
    pub verified_exchanges_1y: i64,
    #[ts(type = "number")]
    pub total_areas: i64,
    #[ts(type = "number")]
    pub verified_areas_1y: i64,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export)]
pub struct ChartEntry {
    pub date: String,
    #[ts(type = "number")]
    pub value: i64,
}

#[get("")]
pub async fn get(pool: Data<MainPool>) -> RestResult<Dashboard> {
    let total_merchants = db::main::element::queries::select_merchants_count(&pool, None)
        .await
        .map_err(|_| RestApiError::database())?;

    let now = OffsetDateTime::now_utc();
    let year_ago = now.date().saturating_sub(Duration::days(365));
    let verified_merchants_1y =
        db::main::element::queries::select_merchants_count(&pool, Some(year_ago))
            .await
            .map_err(|_| RestApiError::database())?;
    let verified_exchanges_1y =
        db::main::element::queries::select_exchanges_count(&pool, Some(year_ago))
            .await
            .map_err(|_| RestApiError::database())?;

    let total_exchanges = db::main::element::queries::select_exchanges_count(&pool, None)
        .await
        .map_err(|_| RestApiError::database())?;

    let total_areas = db::main::area::queries::select_areas_count(&pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let verified_areas_1y =
        db::main::area::queries::select_verified_areas_count(&year_ago.to_string(), &pool)
            .await
            .map_err(|_| RestApiError::database())?;

    let reports = db::main::report::queries::select_by_area_id(662, None, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let total_merchants_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.total_merchants(),
        })
        .collect();

    let verified_merchants_1y_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.up_to_date_elements(),
        })
        .collect();

    let total_exchanges_chart = reports
        .iter()
        .map(|report| ChartEntry {
            date: report.date.to_string(),
            value: report.total_exchanges(),
        })
        .collect();

    Ok(Json(Dashboard {
        total_merchants,
        total_merchants_chart,
        verified_merchants_1y,
        verified_merchants_1y_chart,
        total_exchanges,
        total_exchanges_chart,
        verified_exchanges_1y,
        total_areas,
        verified_areas_1y,
    }))
}

/// Roles allowed to read the analytics dashboard. Mirrors the RPC `dashboard`
/// gate.
fn is_dashboard_privileged(auth: &Auth) -> bool {
    auth.effective_roles()
        .iter()
        .any(|role| matches!(role, Role::Dashboard | Role::Admin | Role::Root))
}

/// Infrastructure/analytics dashboard. Returns the same payload as the RPC
/// `dashboard` method and requires the `dashboard`, `admin` or `root` role.
#[get("/infra")]
pub async fn get_infra(
    auth: Auth,
    main_pool: Data<MainPool>,
    log_pool: Data<LogPool>,
) -> RestResult<analytics_dashboard::Res> {
    if auth.user().is_none() {
        return Err(RestApiError::unauthorized());
    }
    if !is_dashboard_privileged(&auth) {
        return Err(RestApiError::forbidden());
    }
    let res = analytics_dashboard::run(&main_pool, &log_pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(res))
}

#[cfg(test)]
mod test {
    use crate::db::log::test::pool as log_pool;
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::Result;
    use actix_web::http::{header, StatusCode};
    use actix_web::test;
    use actix_web::web::{scope, Data};
    use actix_web::App;

    async fn seed_token(pool: &crate::db::main::MainPool, roles: Vec<Role>) -> Result<String> {
        let user = crate::db::main::user::queries::insert("tester", "", pool).await?;
        let secret = "test-secret".to_string();
        crate::db::main::access_token::queries::insert(
            user.id,
            String::new(),
            secret.clone(),
            roles,
            pool,
        )
        .await?;
        Ok(secret)
    }

    async fn call(
        pool: &crate::db::main::MainPool,
        log_pool: &crate::db::log::LogPool,
        secret: Option<&str>,
    ) -> actix_web::dev::ServiceResponse {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(log_pool.clone()))
                .service(scope("/dashboard").service(super::get_infra)),
        )
        .await;
        let mut request = test::TestRequest::get().uri("/dashboard/infra");
        if let Some(secret) = secret {
            request = request.insert_header((header::AUTHORIZATION, format!("Bearer {secret}")));
        }
        test::call_service(&app, request.to_request()).await
    }

    #[test]
    async fn infra_requires_auth() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        let res = call(&pool, &log_pool, None).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn infra_forbidden_for_regular_user() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        let secret = seed_token(&pool, vec![Role::User]).await?;
        let res = call(&pool, &log_pool, Some(&secret)).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn infra_allowed_for_dashboard_role() -> Result<()> {
        let pool = pool();
        let log_pool = log_pool();
        let secret = seed_token(&pool, vec![Role::Dashboard]).await?;
        let res = call(&pool, &log_pool, Some(&secret)).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(res).await;
        assert!(body.get("logs").is_some());
        assert!(body.get("generation_time_ms").is_some());
        Ok(())
    }
}
