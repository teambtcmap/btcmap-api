use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::Value;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;

#[allow(dead_code)]
pub struct Admin {
    pub id: i64,
    pub name: String,
    pub password: String,
    pub allowed_methods: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

const TABLE: &str = "admin";
const ALL_COLUMNS: &str = "id, name, password, allowed_methods, created_at, updated_at, deleted_at";
const _COL_ID: &str = "id";
const COL_NAME: &str = "name";
const COL_PASSWORD: &str = "password";
const _COL_ALLOWED_METHODS: &str = "allowed_methods";
const _COL_CREATED_AT: &str = "created_at";
const _COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at";

impl Admin {
    pub fn insert(name: &str, password: &str, conn: &Connection) -> Result<Option<Admin>> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_NAME},
                    {COL_PASSWORD}
                ) VALUES (
                    :name,
                    :password
                )
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":name": name,
                ":password": password,
            },
        )?;
        Ok(Admin::select_by_password(password, conn)?)
    }

    pub fn select_by_password(password: &str, conn: &Connection) -> Result<Option<Admin>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_PASSWORD} = :password
            "#
        );
        let res = conn
            .query_row(
                &query,
                named_params! { ":password": password },
                Self::mapper(),
            )
            .optional()?;
        Ok(res)
    }

    const fn mapper() -> fn(&Row) -> rusqlite::Result<Admin> {
        |row: &Row| -> rusqlite::Result<Admin> {
            let allowed_methods: Value = row.get(3)?;
            Ok(Admin {
                id: row.get(0)?,
                name: row.get(1)?,
                password: row.get(2)?,
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