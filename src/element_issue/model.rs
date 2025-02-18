use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, Row};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct ElementIssue {
    pub id: i64,
    pub element_id: i64,
    pub code: String,
    pub severity: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

pub struct SelectOrderedBySeverityRes {
    pub element_osm_type: String,
    pub element_osm_id: i64,
    pub element_name: Option<String>,
    pub issue_code: String,
}

const TABLE_NAME: &str = "element_issue";
const COL_ID: &str = "id";
const COL_ELEMENT_ID: &str = "element_id";
const COL_CODE: &str = "code";
const COL_SEVERITY: &str = "severity";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";
const MAPPER_PROJECTION: &str =
    "id, element_id, code, severity, created_at, updated_at, deleted_at";

impl ElementIssue {
    pub fn insert(
        element_id: i64,
        code: impl Into<String>,
        severity: i64,
        conn: &Connection,
    ) -> Result<ElementIssue> {
        let sql = format!(
            r#"
                INSERT INTO {TABLE_NAME} ({COL_ELEMENT_ID}, {COL_CODE}, {COL_SEVERITY})
                VALUES (:{COL_ELEMENT_ID}, :{COL_CODE}, :{COL_SEVERITY});
            "#
        );
        conn.execute(
            &sql,
            named_params! { ":element_id": element_id, ":code": code.into(), ":severity": severity },
        )?;
        ElementIssue::select_by_id(conn.last_insert_rowid(), conn)
    }

    pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Vec<ElementIssue>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ELEMENT_ID} = :{COL_ELEMENT_ID}
                ORDER BY {COL_UPDATED_AT}, {COL_ID};
            "#
        );
        conn.prepare(&sql)?
            .query_map(named_params! { ":element_id": element_id }, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit;
            "#
        );
        conn.prepare(&sql)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn select_ordered_by_severity_async(
        limit: i64,
        offset: i64,
        pool: &Pool,
    ) -> Result<Vec<SelectOrderedBySeverityRes>> {
        pool.get()
            .await?
            .interact(move |conn| ElementIssue::select_ordered_by_severity(limit, offset, conn))
            .await?
    }

    pub fn select_ordered_by_severity(
        limit: i64,
        offset: i64,
        conn: &Connection,
    ) -> Result<Vec<SelectOrderedBySeverityRes>> {
        let sql = format!(
            r#"
                SELECT json_extract(e.overpass_data, '$.type'), json_extract(e.overpass_data, '$.id'), json_extract(e.overpass_data, '$.tags.name'), ei.{COL_CODE}
                FROM {TABLE_NAME} ei join element e ON e.id = ei.{COL_ELEMENT_ID}
                WHERE ei.{COL_DELETED_AT} IS NULL
                ORDER BY ei.{COL_SEVERITY} DESC
                LIMIT :limit
                OFFSET :offset;
            "#
        );
        conn.prepare(&sql)?
            .query_map(
                named_params! {
                    ":limit": limit,
                    ":offset": offset,
                },
                mapper_select_ordered_by_severity(),
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<ElementIssue> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, mapper())
            .map_err(Into::into)
    }

    pub async fn select_count_async(include_deleted: bool, pool: &Pool) -> Result<i64> {
        pool.get()
            .await?
            .interact(move |conn| ElementIssue::select_count(include_deleted, conn))
            .await?
    }

    pub fn select_count(include_deleted: bool, conn: &Connection) -> Result<i64> {
        let sql = if include_deleted {
            format!(
                r#"
                    SELECT count({COL_ID})
                    FROM {TABLE_NAME};
                "#
            )
        } else {
            format!(
                r#"
                    SELECT count({COL_ID})
                    FROM {TABLE_NAME}
                    WHERE deleted_at IS NULL;
                "#
            )
        };
        let res: rusqlite::Result<i64, _> = conn.query_row(&sql, [], |row| row.get(0));
        res.map_err(Into::into)
    }

    pub fn set_severity(id: i64, severity: i64, conn: &Connection) -> Result<Self> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_SEVERITY} = :{COL_SEVERITY}
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":severity": severity,
            },
        )?;
        Self::select_by_id(id, conn)
    }

    pub fn set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Self> {
        match deleted_at {
            Some(deleted_at) => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE_NAME}
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
                        UPDATE {TABLE_NAME}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(&sql, named_params! { ":id": id })?;
            }
        };
        Self::select_by_id(id, conn)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<ElementIssue> {
    |row: &Row| -> rusqlite::Result<ElementIssue> {
        Ok(ElementIssue {
            id: row.get(0)?,
            element_id: row.get(1)?,
            code: row.get(2)?,
            severity: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}

const fn mapper_select_ordered_by_severity(
) -> fn(&Row) -> rusqlite::Result<SelectOrderedBySeverityRes> {
    |row: &Row| -> rusqlite::Result<SelectOrderedBySeverityRes> {
        Ok(SelectOrderedBySeverityRes {
            element_osm_type: row.get(0)?,
            element_osm_id: row.get(1)?,
            element_name: row.get(2)?,
            issue_code: row.get(3)?,
        })
    }
}
