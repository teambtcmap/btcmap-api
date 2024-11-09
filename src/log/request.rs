use crate::Result;
use rusqlite::{named_params, Connection};

const TABLE_NAME: &str = "request";
const COL_ID: &str = "id";
const COL_DATE: &str = "date";
const COL_IP: &str = "ip";
const COL_PATH: &str = "path";
const COL_QUERY: &str = "query";
const COL_CODE: &str = "code";
const COL_ENTITIES: &str = "entities";
const COL_TIME_NS: &str = "time_ns";

pub fn init(conn: &Connection) -> Result<()> {
    let query = format!(
        r#"
            CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
                {COL_ID} INTEGER PRIMARY KEY NOT NULL,
                {COL_DATE} TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
                {COL_IP} TEXT NOT NULL, 
                {COL_PATH} TEXT NOT NULL, 
                {COL_QUERY} TEXT NOT NULL, 
                {COL_CODE} INTEGER NOT NULL,
                {COL_ENTITIES} INTEGER,
                {COL_TIME_NS} INTEGER NOT NULL
            ) STRICT;
        "#
    );
    conn.execute(&query, [])?;
    Ok(())
}

pub fn insert(
    ip: &str,
    path: &str,
    query: &str,
    code: i64,
    entities: Option<i64>,
    time_ns: i64,
    conn: &Connection,
) -> Result<()> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE_NAME} (
                {COL_IP}, 
                {COL_PATH}, 
                {COL_QUERY},
                {COL_CODE},
                {COL_ENTITIES}, 
                {COL_TIME_NS}
            ) VALUES (
                :{COL_IP}, 
                :{COL_PATH}, 
                :{COL_QUERY}, 
                :{COL_CODE},
                :{COL_ENTITIES}, 
                :{COL_TIME_NS}
             );
         "#
    );
    conn.execute(
        &sql,
        named_params! {
            ":ip": ip,
            ":path": path,
            ":query": query,
            ":code": code,
            ":entities": entities,
            ":time_ns": time_ns,
        },
    )?;
    Ok(())
}
