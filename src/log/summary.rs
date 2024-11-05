use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};

pub struct Summary {
    pub id: i64,
    pub reqests: i64,
    pub entities: i64,
    pub time_ns: i64,
}

const TABLE_NAME: &str = "summary";
const COL_ID: &str = "id";
const COL_DATE: &str = "date";
const COL_IP: &str = "ip";
const COL_ENDPOINT: &str = "endpoint";
const COL_REQUESTS: &str = "requests";
const COL_ENTITIES: &str = "entities";
const COL_TIME_NS: &str = "time_ns";

pub fn init(conn: &Connection) -> Result<()> {
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
    Ok(())
}

pub fn insert(
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

pub fn select(date: &str, ip: &str, endpoint: &str, conn: &Connection) -> Result<Option<Summary>> {
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

pub fn update(
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

const fn mapper() -> fn(&Row) -> rusqlite::Result<Summary> {
    |row: &Row| -> rusqlite::Result<Summary> {
        Ok(Summary {
            id: row.get(0)?,
            reqests: row.get(1)?,
            entities: row.get(2)?,
            time_ns: row.get(3)?,
        })
    }
}
