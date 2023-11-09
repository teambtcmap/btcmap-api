use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use tracing::debug;

pub struct Token {
    pub id: i64,
    pub user_id: i64,
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Token {
    #[cfg(test)]
    pub fn insert(user_id: i64, secret: &str, conn: &Connection) -> Result<Token> {
        let query = format!(
            r#"
                INSERT INTO token (
                    user_id,
                    secret
                ) VALUES (
                    :user_id,
                    :secret
                )
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                    ":user_id": user_id,
                ":secret": secret,
            },
        )?;
        Ok(Token::select_by_secret(secret, conn)?.ok_or(crate::Error::DbTableRowNotFound)?)
    }

    pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<Option<Token>> {
        let query = format!(
            r#"
                SELECT
                    id,
                    user_id,
                    secret,
                    created_at,
                    updated_at,
                    deleted_at
                FROM token
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
                user_id: row.get(1)?,
                secret: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}
