use crate::Result;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::{debug, info};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ElementComment {
    pub id: i64,
    pub element_id: i64,
    pub comment: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "element_comment";
const ALL_COLUMNS: &str = "id, element_id, comment, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_ELEMENT_ID: &str = "element_id";
const COL_COMMENT: &str = "comment";
const COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at ";

impl ElementComment {
    pub fn insert(element_id: i64, comment: &str, conn: &Connection) -> Result<ElementComment> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_ELEMENT_ID},
                    {COL_COMMENT}
                ) VALUES (
                    :element_id,
                    :comment
                )
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":element_id": element_id,
                ":comment": comment,
            },
        )?;
        Ok(ElementComment::select_by_id(conn.last_insert_rowid(), conn)?.unwrap())
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<ElementComment>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded {} element comments in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_latest(limit: i64, conn: &Connection) -> Result<Vec<ElementComment>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                ORDER BY {COL_UPDATED_AT} DESC, {COL_ID} DESC
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":limit": limit,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub fn select_created_between(
        period_start: &OffsetDateTime,
        period_end: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Vec<ElementComment>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_CREATED_AT} > :period_start AND {COL_CREATED_AT} < :period_end
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":period_start": period_start.format(&Rfc3339)?,
                    ":period_end": period_end.format(&Rfc3339)?,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded {} element comments in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Vec<ElementComment>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ELEMENT_ID} = :element_id
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":element_id": element_id,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
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
            comment: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}
