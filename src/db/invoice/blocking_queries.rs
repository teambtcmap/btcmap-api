use super::schema::{self, Columns, Invoice, InvoiceStatus};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use uuid::Uuid;

pub fn insert(
    description: impl Into<String>,
    amount_sats: i64,
    payment_hash: impl Into<String>,
    payment_request: impl Into<String>,
    status: InvoiceStatus,
    conn: &Connection,
) -> Result<Invoice> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {uuid},
                {description},
                {amount_sats},
                {payment_hash},
                {payment_request},
                {status}
            ) VALUES (
                :uuid,
                :description,
                :amount_sats,
                :payment_hash,
                :payment_request,
                :status
            )
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        uuid = Columns::Uuid.as_str(),
        description = Columns::Description.as_str(),
        amount_sats = Columns::AmountSats.as_str(),
        payment_hash = Columns::PaymentHash.as_str(),
        payment_request = Columns::PaymentRequest.as_str(),
        status = Columns::Status.as_str(),
        projection = Invoice::projection(),
    );
    let params = named_params! {
        ":uuid": Uuid::new_v4().to_string(),
        ":amount_sats": amount_sats,
        ":description": description.into(),
        ":payment_hash": payment_hash.into(),
        ":payment_request": payment_request.into(),
        ":status": status,
    };
    conn.query_row(&sql, params, Invoice::mapper())
        .map_err(Into::into)
}

pub fn select_by_status(status: InvoiceStatus, conn: &Connection) -> Result<Vec<Invoice>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {status} = ?1
            ORDER BY {updated_at}, {id}
        "#,
        projection = Invoice::projection(),
        table = schema::TABLE_NAME,
        status = Columns::Status.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![status,], Invoice::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Invoice> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Invoice::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Invoice::mapper())
        .map_err(Into::into)
}

pub fn select_by_uuid(uuid: &str, conn: &Connection) -> Result<Invoice> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {uuid} = ?1
        "#,
        projection = Invoice::projection(),
        table = schema::TABLE_NAME,
        uuid = Columns::Uuid.as_str(),
    );
    conn.query_row(&sql, params![uuid], Invoice::mapper())
        .map_err(Into::into)
}

pub fn set_status(invoice_id: i64, status: InvoiceStatus, conn: &Connection) -> Result<Invoice> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {status} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        status = Columns::Status.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![invoice_id, status])?;
    select_by_id(invoice_id, conn)
}

#[cfg(test)]
mod test {
    use crate::{db::test::conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let invoice = super::insert(
            "desc",
            1,
            "hash",
            "req",
            crate::db::invoice::schema::InvoiceStatus::Unpaid,
            &conn,
        )?;
        assert_eq!(invoice, super::select_by_id(invoice.id, &conn)?);
        Ok(())
    }
}
