use super::schema::{self, Ban, Columns};
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension, ToSql};

pub fn select_by_ip(ip: impl AsRef<str> + ToSql, conn: &Connection) -> Result<Option<Ban>> {
    let sql = format!(
        r#"
            SELECT {projection} 
            FROM {table}
            WHERE {ip} = ?1 AND strftime('%Y-%m-%dT%H:%M:%fZ') > {start_at} AND strftime('%Y-%m-%dT%H:%M:%fZ') < {end_at}
        "#,
        projection = Ban::projection(),
        table = schema::TABLE_NAME,
        ip = Columns::Ip.as_str(),
        start_at = Columns::StartAt.as_str(),
        end_at = Columns::EndAt.as_str(),
    );
    conn.prepare(&sql)?
        .query_row(params![ip], Ban::mapper())
        .optional()
        .map_err(Into::into)
}
