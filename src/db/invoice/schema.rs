use rusqlite::{
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
    Row, ToSql,
};
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "invoice";

pub enum Columns {
    Id,
    Uuid,
    Description,
    AmountSats,
    PaymentHash,
    PaymentRequest,
    Status,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Uuid => "uuid",
            Columns::Description => "description",
            Columns::AmountSats => "amount_sats",
            Columns::PaymentHash => "payment_hash",
            Columns::PaymentRequest => "payment_request",
            Columns::Status => "status",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub struct Invoice {
    pub id: i64,
    pub uuid: String,
    pub description: String,
    pub amount_sats: i64,
    pub payment_hash: String,
    pub payment_request: String,
    pub status: InvoiceStatus,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Invoice {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Uuid,
                Columns::Description,
                Columns::AmountSats,
                Columns::PaymentHash,
                Columns::PaymentRequest,
                Columns::Status,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Invoice> {
        |row: &_| {
            Ok(Invoice {
                id: row.get(Columns::Id.as_str())?,
                uuid: row.get(Columns::Uuid.as_str())?,
                description: row.get(Columns::Description.as_str())?,
                amount_sats: row.get(Columns::AmountSats.as_str())?,
                payment_hash: row.get(Columns::PaymentHash.as_str())?,
                payment_request: row.get(Columns::PaymentRequest.as_str())?,
                status: row.get(Columns::Status.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvoiceStatus {
    Paid,
    Unpaid,
}

impl From<InvoiceStatus> for String {
    fn from(status: InvoiceStatus) -> String {
        match status {
            InvoiceStatus::Paid => "paid".to_string(),
            InvoiceStatus::Unpaid => "unpaid".to_string(),
        }
    }
}

impl TryFrom<&str> for InvoiceStatus {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "paid" => Ok(InvoiceStatus::Paid),
            "unpaid" => Ok(InvoiceStatus::Unpaid),
            _ => Err(format!("Unknown invoice status: {}", value).into()),
        }
    }
}

impl FromSql for InvoiceStatus {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_str()
            .and_then(|s| InvoiceStatus::try_from(s).map_err(|e| FromSqlError::Other(e)))
    }
}

impl ToSql for InvoiceStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(String::from(*self)))
    }
}
