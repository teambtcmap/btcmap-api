use super::{request, summary};
use crate::{data_dir_file, Result};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::StatusCode,
    middleware::Next,
    Error, HttpMessage, HttpRequest,
};
use rusqlite::Connection;
use std::time::Instant;
use time::OffsetDateTime;

thread_local! {
    static CONN: Connection = open_conn().unwrap_or_else(|e| {
        eprintln!("Failed to open logger connection: {e}");
        std::process::exit(1)
    });
}

fn open_conn() -> Result<Connection> {
    let conn = Connection::open(data_dir_file("log.db")?)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    request::init(&conn)?;
    summary::init(&conn)?;
    Ok(conn)
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

pub async fn handle_request(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let started_at = Instant::now();
    let res = next.call(req).await;
    let Ok(res) = res else { return res };
    let extensions = res.request().extensions();
    let entities = extensions.get::<RequestExtension>().map(|it| it.entities);
    drop(extensions);
    let time_ns = Instant::now().duration_since(started_at).as_nanos();
    log_request(
        res.request(),
        res.response().status(),
        entities,
        time_ns as i64,
    )?;
    log_summary(
        res.request(),
        res.request().path(),
        entities.unwrap_or_default(),
        time_ns as i64,
    )?;
    Ok(res)
}

fn log_request(
    req: &HttpRequest,
    code: StatusCode,
    entities: Option<i64>,
    time_ns: i64,
) -> Result<()> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        return Ok(());
    };
    CONN.with(|conn| {
        request::insert(
            addr,
            req.path(),
            req.query_string(),
            code.as_u16() as i64,
            entities,
            time_ns,
            conn,
        )?;
        Ok(())
    })
}

fn log_summary(req: &HttpRequest, endpoint_id: &str, entities: i64, time_ns: i64) -> Result<()> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        return Ok(());
    };
    let today = OffsetDateTime::now_utc().date().to_string();
    CONN.with(|conn| {
        match summary::select(&today, addr, endpoint_id, conn)? {
            Some(entry) => summary::update(
                entry.id,
                entry.reqests + 1,
                entry.entities + entities,
                entry.time_ns + time_ns,
                conn,
            ),
            None => summary::insert(&today, addr, endpoint_id, 1, entities, time_ns, conn),
        }?;
        Ok(())
    })
}
