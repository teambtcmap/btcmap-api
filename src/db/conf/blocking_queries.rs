use super::schema::{self, Conf};
use crate::Result;
use rusqlite::Connection;

pub fn select(conn: &Connection) -> Result<Conf> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
        "#,
        projection = Conf::projection(),
        table = schema::TABLE_NAME,
    );
    conn.prepare(&sql)?
        .query_row((), Conf::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::db::test::conn;

    #[test]
    fn select() -> crate::Result<()> {
        let conn = conn();
        let conf = super::select(&conn)?;
        assert_eq!(conf.paywall_add_element_comment_price_sat, 500);
        Ok(())
    }
}
