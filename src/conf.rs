use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{Connection, Row};

pub struct Conf {
    pub paywall_add_element_comment_price_sat: i64,
    pub paywall_boost_element_30d_price_sat: i64,
    pub paywall_boost_element_90d_price_sat: i64,
    pub paywall_boost_element_365d_price_sat: i64,
}

const TABLE_NAME: &str = "conf";

const MAPPER_PROJECTION: &str = "paywall_add_element_comment_price_sat, paywall_boost_element_30d_price_sat, paywall_boost_element_90d_price_sat, paywall_boost_element_365d_price_sat";

impl Conf {
    pub async fn select_async(pool: &Pool) -> Result<Conf> {
        pool.get()
            .await?
            .interact(|conn| Conf::select(conn))
            .await?
    }

    pub fn select(conn: &Connection) -> Result<Conf> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME};
            "#
        );
        conn.prepare(&sql)?
            .query_row({}, mapper())
            .map_err(Into::into)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Conf> {
    |row: &Row| -> rusqlite::Result<Conf> {
        Ok(Conf {
            paywall_add_element_comment_price_sat: row.get(0)?,
            paywall_boost_element_30d_price_sat: row.get(1)?,
            paywall_boost_element_90d_price_sat: row.get(2)?,
            paywall_boost_element_365d_price_sat: row.get(3)?,
        })
    }
}
