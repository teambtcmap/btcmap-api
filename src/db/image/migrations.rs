use crate::db::image::og::schema::{self, Columns};
use crate::Result;
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
                {col_element_id} INTEGER PRIMARY KEY NOT NULL,
                {col_version} INTEGER NOT NULL,
                {col_image_data} BLOB NOT NULL,
                {col_created_at} TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ'))
            ) STRICT;
        "#,
        table = schema::TABLE_NAME,
        col_element_id = Columns::ElementId.as_str(),
        col_version = Columns::Version.as_str(),
        col_image_data = Columns::ImageData.as_str(),
        col_created_at = Columns::CreatedAt.as_str(),
    );
    conn.execute(&query, [])?;
    conn.execute(
        &format!(
            "CREATE INDEX {table}_{col} ON {table}({col});",
            table = schema::TABLE_NAME,
            col = Columns::CreatedAt.as_str()
        ),
        [],
    )?;
    conn.execute_batch(&format!("PRAGMA user_version={}", 1))?;
    Ok(())
}
