use crate::db::request::{queries, LogPool};
use actix_http::h1;
use actix_web::{
    dev::{self, forward_ready, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    web::{Bytes, Data},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
    time::Instant,
};

pub struct Log;

impl<S: 'static, B> Transform<S, ServiceRequest> for Log
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LogMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LogMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct LogMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for LogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        let pool = req
            .app_data::<Data<LogPool>>()
            .map(|d| d.get_ref().clone())
            .unwrap_or_else(|| panic!("Log pool not configured"));

        Box::pin(async move {
            let started_at = Instant::now();
            let body = req.extract::<Bytes>().await.unwrap();
            let body_str = body.clone();
            let body_str = str::from_utf8(&body_str).ok();
            req.set_payload(bytes_to_payload(body));
            let res = svc.call(req).await?;
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
            let query = res.request().uri().query();
            let response_code = res.status().as_u16() as i64;
            let Some(addr) = addr else {
                return Ok(res);
            };
            queries::insert(
                &addr,
                user_agent.as_deref(),
                None,
                &path,
                query,
                body_str,
                response_code,
                time_ns as i64,
                &pool,
            )
            .await?;
            Ok(res)
        })
    }
}

fn bytes_to_payload(buf: Bytes) -> Payload {
    let (_, mut pl) = h1::Payload::create(true);
    pl.unread_data(buf);
    dev::Payload::from(pl)
}
