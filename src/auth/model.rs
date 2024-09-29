use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::Value;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;

#[allow(dead_code)]
pub struct Token {
    pub id: i64,
    pub owner: String,
    pub secret: String,
    pub allowed_methods: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

const TABLE: &str = "token";
const ALL_COLUMNS: &str = "id, owner, secret, allowed_methods, created_at, updated_at, deleted_at";
const _COL_ID: &str = "id";
const COL_OWNER: &str = "owner";
const COL_SECRET: &str = "secret";
const COL_ALLOWED_METHODS: &str = "allowed_methods";
const _COL_CREATED_AT: &str = "created_at";
const _COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at";

impl Token {
    pub fn insert(
        owner: &str,
        secret: &str,
        allowed_methods: Vec<String>,
        conn: &Connection,
    ) -> Result<Option<Token>> {
        let allowed_methods =
            Value::Array(allowed_methods.into_iter().map(|it| it.into()).collect());
        let query = format!(
            r#"
                INSERT INTO token (
                    {COL_OWNER},
                    {COL_SECRET},
                    {COL_ALLOWED_METHODS}
                ) VALUES (
                    :owner,
                    :secret,
                    :allowed_methods
                )
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":owner": owner,
                ":secret": secret,
                ":allowed_methods": allowed_methods,
            },
        )?;
        Ok(Token::select_by_secret(secret, conn)?)
    }

    pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<Option<Token>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_SECRET} = :secret
            "#
        );
        let res = conn
            .query_row(&query, named_params! { ":secret": secret }, Self::mapper())
            .optional()?;
        Ok(res)
    }

    const fn mapper() -> fn(&Row) -> rusqlite::Result<Token> {
        |row: &Row| -> rusqlite::Result<Token> {
            let allowed_methods: Value = row.get(3)?;
            Ok(Token {
                id: row.get(0)?,
                owner: row.get(1)?,
                secret: row.get(2)?,
                allowed_methods: allowed_methods
                    .as_array()
                    .unwrap()
                    .into_iter()
                    .map(|it| it.as_str().unwrap().into())
                    .collect(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        }
    }
}
