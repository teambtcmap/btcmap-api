use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde::Serialize;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;
use std::time::Instant;
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
const COL_DELETED_AT: &str = "deleted_at ";

impl ElementComment {
    pub async fn insert_async(
        element_id: i64,
        comment: impl Into<String>,
        pool: &Pool,
    ) -> Result<ElementComment> {
        let comment = comment.into();
        pool.get()
            .await?
            .interact(move |conn| ElementComment::insert(element_id, comment, conn))
            .await?
    }

    pub fn insert(
        element_id: i64,
        comment: impl Into<String>,
        conn: &Connection,
    ) -> Result<ElementComment> {
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
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":element_id": element_id,
                ":comment": comment.into(),
            },
        )?;
        Ok(ElementComment::select_by_id(conn.last_insert_rowid(), conn)?.unwrap())
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        include_deleted: bool,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<ElementComment>> {
        let include_deleted_sql = if include_deleted {
            ""
        } else {
            "AND deleted_at IS NULL"
        };
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since {include_deleted_sql}
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

    pub async fn select_by_element_id_async(
        element_id: i64,
        include_deleted: bool,
        limit: i64,
        pool: &Pool,
    ) -> Result<Vec<ElementComment>> {
        pool.get()
            .await?
            .interact(move |conn| {
                Self::select_by_element_id(element_id, include_deleted, limit, conn)
            })
            .await?
    }

    pub fn select_by_element_id(
        element_id: i64,
        include_deleted: bool,
        limit: i64,
        conn: &Connection,
    ) -> Result<Vec<ElementComment>> {
        let include_deleted_sql = if include_deleted {
            ""
        } else {
            "AND deleted_at IS NULL"
        };
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ELEMENT_ID} = :element_id {include_deleted_sql}
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":element_id": element_id,
                    ":limit": limit,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Option<ElementComment>> {
        pool.get()
            .await?
            .interact(move |conn| ElementComment::select_by_id(id, conn))
            .await?
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

    pub async fn set_deleted_at_async(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        pool: &Pool,
    ) -> Result<Option<ElementComment>> {
        pool.get()
            .await?
            .interact(move |conn| ElementComment::set_deleted_at(id, deleted_at, conn))
            .await?
    }

    pub fn set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Option<ElementComment>> {
        match deleted_at {
            Some(deleted_at) => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :{COL_DELETED_AT}
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(
                    &sql,
                    named_params! {
                        ":id": id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(&sql, named_params! { ":id": id })?;
            }
        };
        ElementComment::select_by_id(id, conn)
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
