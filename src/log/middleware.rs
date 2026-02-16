use super::db::{self};
use crate::Result;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    time::Instant,
};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Log;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for Log
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LogMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LogMiddleware { service }))
    }
}

pub struct LogMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        Box::pin(async move {
            let started_at = Instant::now();
            let res = fut.await?;
            let extensions = res.request().extensions();
            let entities = extensions.get::<RequestExtension>().map(|it| it.entities);
            drop(extensions);
            let time_ns = Instant::now().duration_since(started_at).as_nanos();
            let conn_info = res.request().connection_info();
            let Some(addr) = conn_info.realip_remote_addr() else {
                drop(conn_info);
                return Ok(res);
            };
            db::insert(
                addr,
                res.request().headers().get("User-Agent").and_then(|h| h.to_str().ok()),
                res.request().path(),
                res.request().query_string(),
                res.status().as_u16() as i64,
                entities,
                time_ns as i64,
            )?;
            drop(conn_info);
            Ok(res)
        })
    }
}

pub struct RequestExtension {
    pub entities: i64,
}

impl RequestExtension {
    pub fn new(entities: usize) -> Self {
        RequestExtension {
            entities: entities as i64,
        }
    }
}
