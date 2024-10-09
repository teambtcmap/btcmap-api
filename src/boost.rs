use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde::Serialize;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Boost {
    pub id: i64,
    pub admin_id: i64,
    pub element_id: i64,
    pub duration_days: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "boost";
const ALL_COLUMNS: &str =
    "id, admin_id, element_id, duration_days, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_ADMIN_ID: &str = "admin_id";
const COL_ELEMENT_ID: &str = "element_id";
const COL_DURATION_DAYS: &str = "duration_days";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at";

impl Boost {
    pub fn insert(
        admin_id: i64,
        element_id: i64,
        duration_days: i64,
        conn: &Connection,
    ) -> Result<Option<Boost>> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_ADMIN_ID}, {COL_ELEMENT_ID}, {COL_DURATION_DAYS}) 
                VALUES (:admin_id, :element_id, :duration_days)
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(&query, named_params! { ":admin_id": admin_id, ":element_id": element_id, ":duration_days": duration_days })?;
        let res = Boost::select_by_id(conn.last_insert_rowid(), conn)?;
        Ok(res)
    }

    pub fn select_all(conn: &Connection) -> Result<Vec<Boost>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        let res = conn
            .prepare(&query)?
            .query_map({}, mapper())?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Boost>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ID} = :id
            "#
        );
        let res = conn
            .query_row(&query, named_params! { ":id": id }, mapper())
            .optional()?;
        Ok(res)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Boost> {
    |row: &Row| -> rusqlite::Result<Boost> {
        Ok(Boost {
            id: row.get(0)?,
            admin_id: row.get(1)?,
            element_id: row.get(2)?,
            duration_days: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}
