use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use tracing::debug;

#[allow(dead_code)]
pub struct Token {
    pub id: i64,
    pub owner: String,
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Token {
    #[cfg(test)]
    pub fn insert(owner: &str, secret: &str, conn: &Connection) -> Result<Token> {
        use crate::Error;

        let query = format!(
            r#"
                INSERT INTO token (
                    owner,
                    secret
                ) VALUES (
                    :owner,
                    :secret
                )
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":owner": owner,
                ":secret": secret,
            },
        )?;
        Ok(Token::select_by_secret(secret, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<Option<Token>> {
        let query = format!(
            r#"
                SELECT
                    t.id,
                    t.owner,
                    t.secret,
                    t.created_at,
                    t.updated_at,
                    t.deleted_at
                FROM token t
                WHERE secret = :secret
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":secret": secret }, Self::mapper())
            .optional()?)
    }

    const fn mapper() -> fn(&Row) -> rusqlite::Result<Token> {
        |row: &Row| -> rusqlite::Result<Token> {
            Ok(Token {
                id: row.get(0)?,
                owner: row.get(1)?,
                secret: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}
