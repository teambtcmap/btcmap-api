use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::{thread::sleep, time::Duration};
use time::OffsetDateTime;
use tracing::debug;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ElementComment {
    pub id: i64,
    pub element_id: i64,
    pub review: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "element_comment";
const ALL_COLUMNS: &str = "id, element_id, review, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_ELEMENT_ID: &str = "element_id";
const COL_REVIEW: &str = "review";
const _COL_CREATED_AT: &str = "created_at";
const _COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at ";

impl ElementComment {
    pub fn insert(element_id: i64, review: &str, conn: &Connection) -> Result<ElementComment> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_ELEMENT_ID},
                    {COL_REVIEW}
                ) VALUES (
                    :element_id,
                    :review
                )
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":element_id": element_id,
                ":review": review,
            },
        )?;
        Ok(ElementComment::select_by_id(conn.last_insert_rowid(), conn)?.unwrap())
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<ElementComment>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ID} = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":id": id }, mapper())
            .optional()?)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<ElementComment> {
    |row: &Row| -> rusqlite::Result<ElementComment> {
        Ok(ElementComment {
            id: row.get(0)?,
            element_id: row.get(1)?,
            review: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}
