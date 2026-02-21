use crate::{
    db::request::schema::{self, Columns},
    Result,
};
use rusqlite::Connection;

pub fn v0_to_v1(conn: &Connection) -> Result<()> {
    let schema_ver: i16 =
        conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })?;

    if schema_ver != 0 {
        return Ok(());
    }

    let query = format!(
        r#"
            CREATE TABLE IF NOT EXISTS {table} (
                {col_id} INTEGER PRIMARY KEY NOT NULL,
                {col_date} TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
                {col_ip} TEXT NOT NULL,
                {col_user_agent} TEXT,
                {col_user_id} INTEGER,
                {col_path} TEXT NOT NULL, 
                {col_query} TEXT,
                {col_body} TEXT,
                {col_response_code} INTEGER NOT NULL,
                {col_processing_time_ns} INTEGER NOT NULL
            ) STRICT;
        "#,
        table = schema::TABLE_NAME,
        col_id = Columns::Id.as_str(),
        col_date = Columns::Date.as_str(),
        col_ip = Columns::Ip.as_str(),
        col_user_agent = Columns::UserAgent.as_str(),
        col_user_id = Columns::UserId.as_str(),
        col_path = Columns::Path.as_str(),
        col_query = Columns::Query.as_str(),
        col_body = Columns::Body.as_str(),
        col_response_code = Columns::ResponseCode.as_str(),
        col_processing_time_ns = Columns::ProcessingTimeNs.as_str(),
    );
    conn.execute(&query, [])?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::Date.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::Ip.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::UserAgent.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::UserId.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::Path.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::ResponseCode.as_str()
        ),
        [],
    )?;
    conn.execute_batch(&format!("PRAGMA user_version={}", 1))?;
    Ok(())
}
