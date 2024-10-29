use std::time::Instant;

use crate::{data_dir_file, Result};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpMessage, HttpRequest,
};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use time::OffsetDateTime;

#[allow(dead_code)]
struct UsageLog {
    id: i64,
    date: String,
    ip: String,
    endpoint: String,
    reqests: i64,
    entities: i64,
    time_ms: i64,
}

pub struct RequestExtension {
    pub endpoint: String,
    pub entities: i64,
}

impl RequestExtension {
    pub fn new(endpoint: &str, entities: i64) -> Self {
        RequestExtension {
            endpoint: endpoint.into(),
            entities,
        }
    }
}

pub async fn log(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let started_at = Instant::now();
    let res = next.call(req).await;
    let Ok(res) = res else { return res };
    let extensions = res.request().extensions();
    let Some(extension) = extensions.get::<RequestExtension>() else {
        drop(extensions);
        return Ok(res);
    };
    let endpoint = extension.endpoint.clone();
    let entities = extension.entities;
    drop(extensions);
    let time_ms = Instant::now().duration_since(started_at).as_millis() as i64;
    log_sync_api_request(res.request(), &endpoint, entities, time_ms)?;
    Ok(res)
}

fn log_sync_api_request(
    req: &HttpRequest,
    endpoint_id: &str,
    entities: i64,
    time_ms: i64,
) -> Result<()> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        return Ok(());
    };
    let today = OffsetDateTime::now_utc().date().to_string();
    let conn = open_conn()?;
    match select_usage_log(&today, &addr, endpoint_id, &conn)? {
        Some(log) => update_usage_log(
            log.id,
            log.reqests + 1,
            log.entities + entities,
            log.time_ms + time_ms,
            &conn,
        ),
        None => insert_usage_log(&today, &addr, endpoint_id, 1, entities, time_ms, &conn),
    }?;
    Ok(())
}

pub fn open_conn() -> Result<Connection> {
    let conn = Connection::open(data_dir_file("firewall-v1.db")?)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.execute(
        r#"
            CREATE TABLE IF NOT EXISTS usage_log (
                id INTEGER PRIMARY KEY NOT NULL,
                date TEXT NOT NULL,
                ip TEXT NOT NULL, 
                endpoint TEXT NOT NULL, 
                requests INTEGER NOT NULL, 
                entities INTEGER NOT NULL,
                time_ms INTEGER NOT NULL
            ) STRICT;
            CREATE INDEX IF NOT EXISTS usage_log_date_ip_endpoint ON usage_log(date, ip, endpoint);
        "#,
        [],
    )?;
    Ok(conn)
}

fn insert_usage_log(
    date: &str,
    ip: &str,
    endpoint: &str,
    requests: i64,
    entities: i64,
    time_ms: i64,
    conn: &Connection,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO usage_log (
            date, 
            ip, 
            endpoint, 
            requests, 
            entities, 
            time_ms
        ) VALUES (
            :date, 
            :ip, 
            :endpoint, 
            :requests, 
            :entities, 
            :time_ms
         );
         "#,
        named_params! {
            ":date": date,
            ":ip": ip,
            ":endpoint": endpoint,
            ":requests": requests,
            ":entities": entities,
            ":time_ms": time_ms,
        },
    )?;
    Ok(())
}

fn select_usage_log(
    date: &str,
    ip: &str,
    endpoint: &str,
    conn: &Connection,
) -> Result<Option<UsageLog>> {
    let mut stmt = conn.prepare(
        r#"
            SELECT id, date, ip, endpoint, requests, entities, time_ms 
            FROM usage_log
            WHERE date = :date AND ip = :ip and endpoint = :endpoint
        "#,
    )?;
    let res = stmt
        .query_row(
            named_params! {
                ":date": date,
                ":ip": ip,
                ":endpoint": endpoint,
            },
            mapper(),
        )
        .optional()?;
    Ok(res)
}

fn update_usage_log(
    id: i64,
    requests: i64,
    entities: i64,
    time_ms: i64,
    conn: &Connection,
) -> Result<()> {
    conn.execute(
        r#"
            UPDATE usage_log 
            SET requests = :requests, entities = :entities, time_ms = :time_ms
            WHERE id = :id;
         "#,
        named_params! {
            ":id": id,
            ":requests": requests,
            ":entities": entities,
            ":time_ms": time_ms,
        },
    )?;
    Ok(())
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<UsageLog> {
    |row: &Row| -> rusqlite::Result<UsageLog> {
        Ok(UsageLog {
            id: row.get(0)?,
            date: row.get(1)?,
            ip: row.get(2)?,
            endpoint: row.get(3)?,
            reqests: row.get(4)?,
            entities: row.get(5)?,
            time_ms: row.get(6)?,
        })
    }
}
