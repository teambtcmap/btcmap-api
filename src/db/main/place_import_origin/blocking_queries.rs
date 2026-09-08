use super::schema::{self, Columns, ImportOrigin};
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension};

pub fn select_by_name(name: &str, conn: &Connection) -> Result<Option<ImportOrigin>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {name} = ?1
        "#,
        projection = ImportOrigin::projection(),
        table = schema::TABLE_NAME,
        name = Columns::Name.as_ref(),
    );
    conn.prepare(&sql)?
        .query_row(params![name], ImportOrigin::mapper())
        .optional()
        .map_err(Into::into)
}

pub fn select_all(conn: &Connection) -> Result<Vec<ImportOrigin>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {name}
        "#,
        projection = ImportOrigin::projection(),
        table = schema::TABLE_NAME,
        name = Columns::Name.as_ref(),
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], ImportOrigin::mapper())?;
    let mut res = vec![];
    for row in rows {
        res.push(row?);
    }
    Ok(res)
}

#[cfg(test)]
mod test {
    use super::schema::ImportOrigin;
    use crate::{db::main::test::conn, Result};

    #[test]
    fn select_by_name() -> Result<()> {
        let conn = conn();
        let origin = super::select_by_name("square", &conn)?;
        assert_eq!(
            Some(ImportOrigin {
                id: origin.as_ref().unwrap().id,
                name: "square".to_string(),
                gitea_sync_enabled: true,
                gitea_label_id: Some(1307),
            }),
            origin
        );
        Ok(())
    }

    #[test]
    fn select_by_name_missing() -> Result<()> {
        let conn = conn();
        let origin = super::select_by_name("does-not-exist", &conn)?;
        assert_eq!(None, origin);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let origins = super::select_all(&conn)?;
        assert_eq!(5, origins.len());
        Ok(())
    }
}
