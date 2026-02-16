use super::schema::{self, Ban, Columns};
use crate::Result;
use rusqlite::{named_params, params, Connection, OptionalExtension};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

pub fn insert(ip: &str, reason: &str, duration_days: i64, conn: &Connection) -> Result<Ban> {
    let start_at = OffsetDateTime::now_utc();
    let end_at = start_at.saturating_add(Duration::days(duration_days));

    let sql = format!(
        r#"
            INSERT INTO {table} (
                {ip},
                {reason},
                {start_at},
                {end_at}
            ) VALUES (
                :ip,
                :reason,
                :start_at,
                :end_at
            )
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        ip = Columns::Ip.as_str(),
        reason = Columns::Reason.as_str(),
        start_at = Columns::StartAt.as_str(),
        end_at = Columns::EndAt.as_str(),
        projection = Ban::projection(),
    );
    let params = named_params! {
        ":ip": ip,
        ":reason": reason,
        ":start_at": start_at.format(&Rfc3339)?,
        ":end_at": end_at.format(&Rfc3339)?,
    };
    conn.query_row(&sql, params, Ban::mapper())
        .map_err(Into::into)
}

pub fn select_by_ip(ip: &str, conn: &Connection) -> Result<Option<Ban>> {
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

#[cfg(test)]
mod test {
    use crate::{db::test::conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let ip = "127.0.0.1";
        let ban = super::insert(ip, "test", 1, &conn)?;
        assert_eq!(Some(ban), super::select_by_ip(ip, &conn)?);
        Ok(())
    }
}
