use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, Row};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Invoice {
    pub id: i64,
    pub amount_sats: i64,
    pub payment_hash: String,
    pub payment_request: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "invoice";
const ALL_COLUMNS: &str =
    "id, amount_sats, payment_hash, payment_request, status, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_AMOUNT_SATS: &str = "amount_sats";
const COL_PAYMENT_HASH: &str = "payment_hash";
const COL_PAYMENT_REQUEST: &str = "payment_request";
const COL_STATUS: &str = "status";
const COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at ";

impl Invoice {
    pub async fn insert_async(
        amount_sats: i64,
        payment_hash: impl Into<String>,
        payment_request: impl Into<String>,
        status: impl Into<String>,
        pool: &Pool,
    ) -> Result<Invoice> {
        let payment_hash = payment_hash.into();
        let payment_request = payment_request.into();
        let status = status.into();
        pool.get()
            .await?
            .interact(move |conn| {
                Invoice::insert(amount_sats, payment_hash, payment_request, status, conn)
            })
            .await?
    }

    pub fn insert(
        amount_sats: i64,
        payment_hash: impl Into<String>,
        payment_request: impl Into<String>,
        status: impl Into<String>,
        conn: &Connection,
    ) -> Result<Invoice> {
        let sql = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_AMOUNT_SATS},
                    {COL_PAYMENT_HASH},
                    {COL_PAYMENT_REQUEST},
                    {COL_STATUS}
                ) VALUES (
                    :amount_sats,
                    :payment_hash,
                    :payment_request,
                    :status
                )
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":amount_sats": amount_sats,
                ":payment_hash": payment_hash.into(),
                ":payment_request": payment_request.into(),
                ":status": status.into(),
            },
        )?;
        Invoice::select_by_id(conn.last_insert_rowid(), conn)
    }

    pub async fn select_by_status_async(
        status: impl Into<String>,
        pool: &Pool,
    ) -> Result<Vec<Invoice>> {
        let status = status.into();
        pool.get()
            .await?
            .interact(move |conn| Invoice::select_by_status(status, conn))
            .await?
    }

    pub fn select_by_status(status: impl Into<String>, conn: &Connection) -> Result<Vec<Invoice>> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_STATUS} = :status
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        let res = conn
            .prepare(&sql)?
            .query_map(
                named_params! {
                    ":status": status.into(),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Invoice> {
        pool.get()
            .await?
            .interact(move |conn| Invoice::select_by_id(id, conn))
            .await?
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Invoice> {
        let sql = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ID} = :id
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, mapper())
            .map_err(Into::into)
    }

    pub async fn set_status_async(
        invoice_id: i64,
        status: impl Into<String>,
        pool: &Pool,
    ) -> Result<Invoice> {
        let status = status.into();
        pool.get()
            .await?
            .interact(move |conn| Invoice::set_status(invoice_id, status, conn))
            .await?
    }

    pub fn set_status(
        invoice_id: i64,
        status: impl Into<String>,
        conn: &Connection,
    ) -> Result<Invoice> {
        let sql = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_STATUS} = :{COL_STATUS}
                WHERE {COL_ID} = :{COL_ID}
        "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": invoice_id,
                ":status": status.into(),
            },
        )?;
        Invoice::select_by_id(invoice_id, conn)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Invoice> {
    |row: &Row| -> rusqlite::Result<Invoice> {
        Ok(Invoice {
            id: row.get(0)?,
            amount_sats: row.get(1)?,
            payment_hash: row.get(2)?,
            payment_request: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}
