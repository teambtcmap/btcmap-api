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
    use crate::db::main::test::conn;

    #[test]
    fn select() -> crate::Result<()> {
        let conn = conn();
        let conf = super::select(&conn)?;
        assert_eq!(conf.paywall_add_element_comment_price_sat, 500);
        assert_eq!(conf.boost_element_prices, vec![]);
        Ok(())
    }

    #[test]
    fn select_with_boost_prices() -> crate::Result<()> {
        let conn = conn();
        conn.execute(
            "UPDATE conf SET boost_element_prices = ?1",
            rusqlite::params![
                r#"[{"days":30,"sats":5000},{"days":90,"sats":10000},{"days":365,"sats":30000}]"#
            ],
        )?;
        let conf = super::select(&conn)?;
        assert_eq!(
            conf.boost_element_prices,
            vec![
                super::schema::BoostPrice {
                    days: 30,
                    sats: 5000
                },
                super::schema::BoostPrice {
                    days: 90,
                    sats: 10000
                },
                super::schema::BoostPrice {
                    days: 365,
                    sats: 30000
                },
            ]
        );
        Ok(())
    }
}
