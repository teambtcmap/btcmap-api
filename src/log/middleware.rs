use super::db::{self, LogPool};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error as ActixError,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    time::Instant,
};

pub struct Log;

impl<S, B> Transform<S, ServiceRequest> for Log
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = LogMiddleware<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LogMiddleware { service }))
    }
}

pub struct LogMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let pool = req
            .app_data::<Data<LogPool>>()
            .map(|d| d.get_ref().clone())
            .unwrap_or_else(|| panic!("Log pool not configured"));
        let fut = self.service.call(req);
        Box::pin(async move {
            let started_at = Instant::now();
            let res = fut.await?;
            let time_ns = Instant::now().duration_since(started_at).as_nanos();
            let addr = res
                .request()
                .connection_info()
                .realip_remote_addr()
                .map(|s| s.to_owned());
            let user_agent = res
                .request()
                .headers()
                .get("User-Agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_owned());
            let path = res.request().path().to_owned();
            let query = res.request().query_string().to_owned();
            let code = res.status().as_u16() as i64;
            let Some(addr) = addr else {
                return Ok(res);
            };
            db::insert(
                &addr,
                user_agent.as_deref(),
                &path,
                &query,
                code,
                time_ns as i64,
                &pool,
            )
            .await?;
            Ok(res)
        })
    }
}
