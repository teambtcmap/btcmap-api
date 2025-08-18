use crate::db;
use actix_web::web::Data;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error,
};
use deadpool_sqlite::Pool;

#[allow(clippy::await_holding_refcell_ref)]
pub async fn check_if_banned(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> std::result::Result<ServiceResponse<impl MessageBody>, Error> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        drop(conn_info);
        return next.call(req).await;
    };
    let Some(pool) = req.app_data::<Data<Pool>>() else {
        drop(conn_info);
        return next.call(req).await;
    };
    let addr = addr.to_owned();
    let current_ban = db::ban::queries::select_by_ip(addr, pool).await?;
    match current_ban {
        Some(current_ban) => {
            Err(actix_web::error::ErrorForbidden(format!(
                "You are banned for the following reason: {}. You can contact us by the following email for more details: support@btcmap.org.",
                current_ban.reason,
            )))?
        }
        None => {
            drop(conn_info);
            next.call(req).await
        }
    }
}
