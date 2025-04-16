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
pub struct AreaElement {
    pub id: i64,
    pub area_id: i64,
    pub element_id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "area_element";
const ALL_COLUMNS: &str = "id, area_id, element_id, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_AREA_ID: &str = "area_id";
const COL_ELEMENT_ID: &str = "element_id";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at ";

impl AreaElement {
    pub async fn insert_bulk_async(area_id: i64, element_ids: Vec<i64>, pool: &Pool) -> Result<()> {
        pool.get()
            .await?
            .interact(move |conn| Self::insert_bulk(area_id, element_ids, conn))
            .await?
    }

    pub fn insert_bulk(area_id: i64, element_ids: Vec<i64>, conn: &mut Connection) -> Result<()> {
        let sp = conn.savepoint()?;
        for element in element_ids {
            AreaElement::insert(area_id, element, &sp)?;
        }
        sp.commit()?;
        Ok(())
    }

    pub async fn insert_async(area_id: i64, element_id: i64, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::insert(area_id, element_id, conn))
            .await?
    }

    pub fn insert(area_id: i64, element_id: i64, conn: &Connection) -> Result<AreaElement> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_AREA_ID},
                    {COL_ELEMENT_ID}
                ) VALUES (
                    :area_id,
                    :element_id
                )
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":area_id": area_id,
                ":element_id": element_id,
            },
        )?;
        Ok(AreaElement::select_by_id(conn.last_insert_rowid(), conn)?.unwrap())
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<AreaElement>> {
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
            "Loaded {} area_elements in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub async fn select_by_area_id_and_element_id_async(
        area_id: i64,
        element_id: i64,
        pool: &Pool,
    ) -> Result<Option<Self>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_area_id_and_element_id(area_id, element_id, conn))
            .await?
    }

    pub fn select_by_area_id_and_element_id(
        area_id: i64,
        element_id: i64,
        conn: &Connection,
    ) -> Result<Option<AreaElement>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_AREA_ID} = :area_id AND {COL_ELEMENT_ID} = :element_id;
            "#
        );
        Ok(conn
            .query_row(
                &query,
                named_params! { ":area_id": area_id, ":element_id": element_id },
                mapper(),
            )
            .optional()?)
    }

    pub async fn select_by_area_id_async(area_id: i64, pool: &Pool) -> Result<Vec<Self>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_area_id(area_id, conn))
            .await?
    }

    pub fn select_by_area_id(area_id: i64, conn: &Connection) -> Result<Vec<AreaElement>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_AREA_ID} = :area_id
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":area_id": area_id,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub async fn select_by_element_id_async(element_id: i64, pool: &Pool) -> Result<Vec<Self>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_element_id(element_id, conn))
            .await?
    }

    pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Vec<AreaElement>> {
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

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Option<Self>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_id(id, conn))
            .await?
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<AreaElement>> {
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
    ) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_deleted_at(id, deleted_at, conn))
            .await?
    }

    pub fn set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Self> {
        match deleted_at {
            Some(deleted_at) => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :deleted_at
                        WHERE {COL_ID} = :id
                    "#
                );
                debug!(query);
                #[cfg(not(test))]
                sleep(Duration::from_millis(10));
                conn.execute(
                    &query,
                    named_params! {
                        ":id": id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ID} = :id
                    "#
                );
                debug!(query);
                #[cfg(not(test))]
                sleep(Duration::from_millis(10));
                conn.execute(&query, named_params! { ":id": id })?;
            }
        };
        Ok(AreaElement::select_by_id(id, conn)?.unwrap())
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<AreaElement> {
    |row: &Row| -> rusqlite::Result<AreaElement> {
        Ok(AreaElement {
            id: row.get(0)?,
            area_id: row.get(1)?,
            element_id: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}
