use super::schema::Columns;
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension};

pub fn select(key: &str, conn: &Connection) -> Result<Option<String>> {
    let sql = format!(
        r#"
            SELECT {value}
            FROM {table}
            WHERE {key} = ?1
        "#,
        value = Columns::Value.as_ref(),
        table = super::schema::TABLE_NAME,
        key = Columns::Key.as_ref(),
    );
    conn.prepare(&sql)?
        .query_row(params![key], |row| row.get(0))
        .optional()
        .map_err(Into::into)
}

pub fn upsert(key: &str, value: &str, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({key}, {value})
            VALUES (?1, ?2)
            ON CONFLICT ({key}) DO UPDATE SET {value} = excluded.{value}
        "#,
        table = super::schema::TABLE_NAME,
        key = Columns::Key.as_ref(),
        value = Columns::Value.as_ref(),
    );
    conn.execute(&sql, params![key, value])?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::main::test::conn;
    use crate::Result;

    #[test]
    fn roundtrip() -> Result<()> {
        let conn = conn();
        assert_eq!(None, super::select("k", &conn)?);
        super::upsert("k", "v1", &conn)?;
        assert_eq!(Some("v1".to_string()), super::select("k", &conn)?);
        super::upsert("k", "v2", &conn)?;
        assert_eq!(Some("v2".to_string()), super::select("k", &conn)?);
        Ok(())
    }
}
