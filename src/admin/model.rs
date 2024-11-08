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
    pub allowed_actions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

const TABLE: &str = "admin";
const ALL_COLUMNS: &str = "id, name, password, allowed_methods, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_NAME: &str = "name";
const COL_PASSWORD: &str = "password";
const COL_ALLOWED_METHODS: &str = "allowed_methods";
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
        Admin::select_by_password(password, conn)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Admin>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ID} = :id
            "#
        );
        let res = conn
            .query_row(&query, named_params! { ":id": id }, Self::mapper())
            .optional()?;
        Ok(res)
    }

    pub fn select_by_name(name: &str, conn: &Connection) -> Result<Option<Admin>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_NAME} = :name
            "#
        );
        let res = conn
            .query_row(&query, named_params! { ":name": name }, Self::mapper())
            .optional()?;
        Ok(res)
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

    pub fn set_allowed_actions(
        id: i64,
        allowed_actions: &Vec<String>,
        conn: &Connection,
    ) -> Result<Option<Admin>> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_ALLOWED_METHODS} = json(:allowed_actions)
                WHERE {COL_ID} = :id
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":allowed_actions": serde_json::to_string(allowed_actions)?,
            },
        )?;
        Admin::select_by_id(id, conn)
    }

    const fn mapper() -> fn(&Row) -> rusqlite::Result<Admin> {
        |row: &Row| -> rusqlite::Result<Admin> {
            let allowed_actions: Value = row.get(3)?;
            Ok(Admin {
                id: row.get(0)?,
                name: row.get(1)?,
                password: row.get(2)?,
                allowed_actions: allowed_actions
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|it| it.as_str().unwrap().into())
                    .collect(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        }
    }
}
