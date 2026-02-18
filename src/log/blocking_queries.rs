use crate::Result;
use rusqlite::{named_params, Connection};

pub fn insert(
    ip: &str,
    user_agent: Option<&str>,
    path: &str,
    query: &str,
    code: i64,
    time_ns: i64,
    conn: &Connection,
) -> Result<()> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE_NAME} (
                {COL_IP},
                {COL_USER_AGENT},
                {COL_PATH}, 
                {COL_QUERY},
                {COL_CODE},
                {COL_TIME_NS}
            ) VALUES (
                :{COL_IP},
                :{COL_USER_AGENT},
                :{COL_PATH}, 
                :{COL_QUERY}, 
                :{COL_CODE},
                :{COL_TIME_NS}
             );
         "#
    );
    conn.execute(
        &sql,
        named_params! {
            ":ip": ip,
            ":user_agent": user_agent,
            ":path": path,
            ":query": query,
            ":code": code,
            ":time_ns": time_ns,
        },
    )?;
    Ok(())
}

const TABLE_NAME: &str = "request";
const COL_ID: &str = "id";
const COL_DATE: &str = "date";
const COL_IP: &str = "ip";
const COL_USER_AGENT: &str = "user_agent";
const COL_PATH: &str = "path";
const COL_QUERY: &str = "query";
const COL_CODE: &str = "code";
const COL_TIME_NS: &str = "time_ns";

pub fn migrate(conn: &Connection) -> Result<()> {
    let query = format!(
        r#"
            CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
                {COL_ID} INTEGER PRIMARY KEY NOT NULL,
                {COL_DATE} TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
                {COL_IP} TEXT NOT NULL,
                {COL_USER_AGENT} TEXT,
                {COL_PATH} TEXT NOT NULL, 
                {COL_QUERY} TEXT NOT NULL, 
                {COL_CODE} INTEGER NOT NULL,
                {COL_TIME_NS} INTEGER NOT NULL
            ) STRICT;
        "#
    );
    conn.execute(&query, [])?;
    conn.execute(
        &format!("CREATE INDEX IF NOT EXISTS {TABLE_NAME}_{COL_DATE} ON {TABLE_NAME}({COL_DATE});"),
        [],
    )?;
    conn.execute(
        &format!("CREATE INDEX IF NOT EXISTS {TABLE_NAME}_{COL_CODE} ON {TABLE_NAME}({COL_CODE});"),
        [],
    )?;
    Ok(())
}
