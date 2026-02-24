use crate::{db::log::request, Result};
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
        table = request::schema::TABLE_NAME,
        col_id = request::schema::Columns::Id.as_str(),
        col_date = request::schema::Columns::Date.as_str(),
        col_ip = request::schema::Columns::Ip.as_str(),
        col_user_agent = request::schema::Columns::UserAgent.as_str(),
        col_user_id = request::schema::Columns::UserId.as_str(),
        col_path = request::schema::Columns::Path.as_str(),
        col_query = request::schema::Columns::Query.as_str(),
        col_body = request::schema::Columns::Body.as_str(),
        col_response_code = request::schema::Columns::ResponseCode.as_str(),
        col_processing_time_ns = request::schema::Columns::ProcessingTimeNs.as_str(),
    );
    conn.execute(&query, [])?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::Date.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::Ip.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::UserAgent.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::UserId.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::Path.as_str()
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = request::schema::TABLE_NAME,
            col = request::schema::Columns::ResponseCode.as_str()
        ),
        [],
    )?;
    conn.execute_batch(&format!("PRAGMA user_version={}", 1))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::log::request::schema::TABLE_NAME;
    use crate::db::log::test::conn;

    #[test]
    fn v0_to_v1_creates_table() -> crate::Result<()> {
        let conn = conn();

        let table_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [TABLE_NAME],
            |row| row.get(0),
        )?;
        assert_eq!(table_exists, 1);

        Ok(())
    }

    #[test]
    fn v0_to_v1_creates_indexes() -> crate::Result<()> {
        let conn = conn();

        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'request_%'",
        )?;
        let index_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        assert!(index_names.iter().any(|n| n.contains("date")));
        assert!(index_names.iter().any(|n| n.contains("ip")));
        assert!(index_names.iter().any(|n| n.contains("user_agent")));
        assert!(index_names.iter().any(|n| n.contains("user_id")));
        assert!(index_names.iter().any(|n| n.contains("path")));
        assert!(index_names.iter().any(|n| n.contains("response_code")));

        Ok(())
    }

    #[test]
    fn v0_to_v1_idempotent() -> crate::Result<()> {
        let conn = conn();
        super::v0_to_v1(&conn)?;

        let table_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [TABLE_NAME],
            |row| row.get(0),
        )?;
        assert_eq!(table_exists, 1);

        Ok(())
    }

    #[test]
    fn v0_to_v1_sets_version() -> crate::Result<()> {
        let conn = conn();

        let version: i16 =
            conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })?;
        assert_eq!(version, 1);

        Ok(())
    }
}
