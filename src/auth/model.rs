use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use tracing::debug;

pub struct Token {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Token {
    #[cfg(test)]
    pub fn insert(user_id: i64, secret: &str, conn: &Connection) -> Result<Token> {
        use crate::Error;

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
        Ok(Token::select_by_secret(secret, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<Option<Token>> {
        let query = format!(
            r#"
                SELECT
                    t.id,
                    t.user_id,
                    json_extract(u.osm_data, '$.display_name'),
                    t.secret,
                    t.created_at,
                    t.updated_at,
                    t.deleted_at
                FROM token t
                JOIN user u on u.id = user_id
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
                user_name: row.get(2)?,
                secret: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        }
    }
}
