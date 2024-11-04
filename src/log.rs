use crate::{data_dir_file, Result};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpMessage, HttpRequest,
};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use std::time::Instant;
use time::OffsetDateTime;

thread_local! {
    static CONN: Connection = open_conn().unwrap_or_else(|e| {
        eprintln!("Failed to open logger connection: {e}");
        std::process::exit(1)
    });
}

struct LogEntry {
    id: i64,
    reqests: i64,
    entities: i64,
    time_ns: i64,
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
    let time_ns = Instant::now().duration_since(started_at).as_nanos();
    _log(res.request(), &endpoint, entities, time_ns as i64)?;
    Ok(res)
}

fn _log(req: &HttpRequest, endpoint_id: &str, entities: i64, time_ns: i64) -> Result<()> {
    let conn_info = req.connection_info();
    let Some(addr) = conn_info.realip_remote_addr() else {
        return Ok(());
    };
    let today = OffsetDateTime::now_utc().date().to_string();
    CONN.with(|conn| {
        match select_entry(&today, &addr, endpoint_id, &conn)? {
            Some(entry) => update_entry(
                entry.id,
                entry.reqests + 1,
                entry.entities + entities,
                entry.time_ns + time_ns,
                &conn,
            ),
            None => insert_entry(&today, &addr, endpoint_id, 1, entities, time_ns, &conn),
        }?;
        Ok(())
    })
}

const TABLE_NAME: &str = "summary";
const COL_ID: &str = "id";
const COL_DATE: &str = "date";
const COL_IP: &str = "ip";
const COL_ENDPOINT: &str = "endpoint";
const COL_REQUESTS: &str = "requests";
const COL_ENTITIES: &str = "entities";
const COL_TIME_NS: &str = "time_ns";

pub fn open_conn() -> Result<Connection> {
    let conn = Connection::open(data_dir_file("log.db")?)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    let query = format!(
        r#"
            CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
                {COL_ID} INTEGER PRIMARY KEY NOT NULL,
                {COL_DATE} TEXT NOT NULL,
                {COL_IP} TEXT NOT NULL,
                {COL_ENDPOINT} TEXT NOT NULL, 
                {COL_REQUESTS} INTEGER NOT NULL, 
                {COL_ENTITIES} INTEGER NOT NULL,
                {COL_TIME_NS} INTEGER NOT NULL
            ) STRICT;
            CREATE UNIQUE INDEX IF NOT EXISTS {COL_DATE}_{COL_IP}_{COL_ENDPOINT} ON {TABLE_NAME}({COL_DATE}, {COL_IP}, {COL_ENDPOINT});
        "#
    );
    conn.execute(&query, [])?;
    Ok(conn)
}

fn insert_entry(
    date: &str,
    ip: &str,
    endpoint: &str,
    requests: i64,
    entities: i64,
    time_ns: i64,
    conn: &Connection,
) -> Result<()> {
    let query = format!(
        r#"
            INSERT INTO {TABLE_NAME} (
                {COL_DATE}, 
                {COL_IP}, 
                {COL_ENDPOINT}, 
                {COL_REQUESTS}, 
                {COL_ENTITIES}, 
                {COL_TIME_NS}
            ) VALUES (
                :{COL_DATE}, 
                :{COL_IP}, 
                :{COL_ENDPOINT}, 
                :{COL_REQUESTS}, 
                :{COL_ENTITIES}, 
                :{COL_TIME_NS}
             );
         "#
    );
    conn.execute(
        &query,
        named_params! {
            ":date": date,
            ":ip": ip,
            ":endpoint": endpoint,
            ":requests": requests,
            ":entities": entities,
            ":time_ns": time_ns,
        },
    )?;
    Ok(())
}

fn select_entry(
    date: &str,
    ip: &str,
    endpoint: &str,
    conn: &Connection,
) -> Result<Option<LogEntry>> {
    let mut stmt = conn.prepare(&format!(
        r#"
            SELECT {COL_ID}, {COL_REQUESTS}, {COL_ENTITIES}, {COL_TIME_NS} 
            FROM {TABLE_NAME}
            WHERE {COL_DATE} = :{COL_DATE} AND {COL_IP} = :{COL_IP} AND {COL_ENDPOINT} = :{COL_ENDPOINT}
        "#
    ))?;
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

fn update_entry(
    id: i64,
    requests: i64,
    entities: i64,
    time_ns: i64,
    conn: &Connection,
) -> Result<()> {
    let query = format!(
        r#"
            UPDATE {TABLE_NAME} 
            SET {COL_REQUESTS} = :{COL_REQUESTS}, {COL_ENTITIES} = :{COL_ENTITIES}, {COL_TIME_NS} = :{COL_TIME_NS}
            WHERE {COL_ID} = :{COL_ID};
         "#
    );
    conn.execute(
        &query,
        named_params! {
            ":id": id,
            ":requests": requests,
            ":entities": entities,
            ":time_ns": time_ns,
        },
    )?;
    Ok(())
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<LogEntry> {
    |row: &Row| -> rusqlite::Result<LogEntry> {
        Ok(LogEntry {
            id: row.get(0)?,
            reqests: row.get(1)?,
            entities: row.get(2)?,
            time_ns: row.get(3)?,
        })
    }
}
