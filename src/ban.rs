use crate::Result;
use actix_web::web::Data;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error,
};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use std::sync::Arc;
use time::OffsetDateTime;

#[allow(dead_code)]
struct Ban {
    id: i64,
    ip: String,
    reason: String,
    start_at: OffsetDateTime,
    end_at: OffsetDateTime,
}

pub async fn check_if_banned(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> std::result::Result<ServiceResponse<impl MessageBody>, Error> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        drop(conn_info);
        return next.call(req).await;
    };
    let Some(pool) = req.app_data::<Data<Arc<Pool>>>() else {
        drop(conn_info);
        return next.call(req).await;
    };
    let addr = addr.to_owned();
    let current_ban = pool
        .get()
        .await
        .unwrap()
        .interact(move |conn| select_ban_by_ip(&addr, conn))
        .await
        .unwrap()?;
    match current_ban {
        Some(current_ban) => {
            return Err(actix_web::error::ErrorForbidden(format!(
                "You are banned for the following reason: {}",
                current_ban.reason,
            )))?
        }
        None => {
            drop(conn_info);
            next.call(req).await
        }
    }
}

fn select_ban_by_ip(ip: &str, conn: &Connection) -> Result<Option<Ban>> {
    let mut stmt = conn.prepare(
        r#"
            SELECT id, ip, reason, start_at, end_at 
            FROM ban
            WHERE ip = :ip AND strftime('%Y-%m-%dT%H:%M:%fZ') > start_at AND strftime('%Y-%m-%dT%H:%M:%fZ') < end_at
        "#,
    )?;
    let res = stmt
        .query_row(named_params! { ":ip": ip }, mapper())
        .optional()?;
    Ok(res)
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Ban> {
    |row: &Row| -> rusqlite::Result<Ban> {
        Ok(Ban {
            id: row.get(0)?,
            ip: row.get(1)?,
            reason: row.get(2)?,
            start_at: row.get(3)?,
            end_at: row.get(4)?,
        })
    }
}
