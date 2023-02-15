use rusqlite::Result;
use rusqlite::Row;

pub struct Token {
    pub id: i64,
    pub user_id: i64,
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

#[cfg(test)]
pub static INSERT: &str = r#"
    INSERT INTO token (
        user_id,
        secret
    ) VALUES (
        :user_id,
        :secret
    )
"#;

pub static SELECT_BY_SECRET: &str = r#"
    SELECT
        id,
        user_id,
        secret,
        created_at,
        updated_at,
        deleted_at
    FROM token
    WHERE secret = :secret
"#;

pub static SELECT_BY_SECRET_MAPPER: fn(&Row) -> Result<Token> = full_mapper();

const fn full_mapper() -> fn(&Row) -> Result<Token> {
    |row: &Row| -> Result<Token> {
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
