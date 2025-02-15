use crate::Result;
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
