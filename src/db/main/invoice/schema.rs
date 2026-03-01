use rusqlite::{
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
    Row, ToSql,
};
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "invoice";

pub enum Columns {
    Id,
    Uuid,
    Source,
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
            Columns::Source => "source",
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
    pub source: String,
    pub description: String,
    pub amount_sats: i64,
    pub payment_hash: String,
    pub payment_request: String,
    pub status: InvoiceStatus,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvoicedService {
    Boost { element_id: i64, duration_days: i64 },
    Comment { comment_id: i64 },
    Unknown(String),
}

impl InvoicedService {
    pub fn from_description(description: &str) -> Self {
        let parts: Vec<&str> = description.split(':').collect();

        match parts.as_slice() {
            ["element_boost", id_str, duration_str] => {
                match (id_str.parse::<i64>(), duration_str.parse::<i64>()) {
                    (Ok(element_id), Ok(duration_days)) => InvoicedService::Boost {
                        element_id,
                        duration_days,
                    },
                    _ => InvoicedService::Unknown(description.to_string()),
                }
            }
            ["element_comment", id_str, _] => match id_str.parse::<i64>() {
                Ok(comment_id) => InvoicedService::Comment { comment_id },
                _ => InvoicedService::Unknown(description.to_string()),
            },
            _ => InvoicedService::Unknown(description.to_string()),
        }
    }

    #[cfg(test)]
    pub fn to_description(&self) -> String {
        match self {
            InvoicedService::Boost {
                element_id,
                duration_days,
            } => {
                format!("element_boost:{}:{}", element_id, duration_days)
            }
            InvoicedService::Comment { comment_id } => {
                format!("element_comment:{}:{}", comment_id, "publish")
            }
            InvoicedService::Unknown(desc) => desc.clone(),
        }
    }
}

impl Invoice {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Uuid,
                Columns::Source,
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
                source: row.get(Columns::Source.as_str())?,
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

    pub fn service(&self) -> InvoicedService {
        InvoicedService::from_description(&self.description)
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
            .and_then(|s| InvoiceStatus::try_from(s).map_err(FromSqlError::Other))
    }
}

impl ToSql for InvoiceStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(String::from(*self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boost_parse() {
        let description = "element_boost:2769:30";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Boost {
                element_id: 2769,
                duration_days: 30
            }
        );

        assert_eq!(service.to_description(), description);
    }

    #[test]
    fn boost_parse_with_negative_numbers() {
        let description = "element_boost:-100:-30";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Boost {
                element_id: -100,
                duration_days: -30
            }
        );

        assert_eq!(service.to_description(), description);
    }

    #[test]
    fn boost_parse_large_numbers() {
        let description = "element_boost:999999:365";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Boost {
                element_id: 999999,
                duration_days: 365
            }
        );

        assert_eq!(service.to_description(), description);
    }

    #[test]
    fn boost_parse_invalid_number() {
        let description = "element_boost:abc:30";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Unknown("element_boost:abc:30".to_string())
        );

        assert_eq!(service.to_description(), description);
    }

    #[test]
    fn boost_parse_missing_parts() {
        let description = "element_boost:2769";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Unknown("element_boost:2769".to_string())
        );
    }

    #[test]
    fn comment_parse_valid() {
        let test_cases = vec![
            ("element_comment:1870:publish", 1870),
            ("element_comment:1234:edit", 1234),
            ("element_comment:5678:delete", 5678),
            ("element_comment:999:anything_here", 999),
        ];

        for (description, expected_id) in test_cases {
            let service = InvoicedService::from_description(description);

            assert_eq!(
                service,
                InvoicedService::Comment {
                    comment_id: expected_id
                }
            );

            assert_eq!(
                service.to_description(),
                format!("element_comment:{}:publish", expected_id)
            );
        }
    }

    #[test]
    fn comment_parse_invalid_id() {
        let description = "element_comment:not_a_number:publish";
        let service = InvoicedService::from_description(description);

        assert_eq!(
            service,
            InvoicedService::Unknown("element_comment:not_a_number:publish".to_string())
        );
    }

    #[test]
    fn unknown_service_type() {
        let test_cases = vec![
            "unknown_service:123:456",
            "element_like:100:200",
            "boost:123:30",
            "element_boost",
            "element_boost:123:456:extra",
            "",
            "just_one_part",
            "two:parts",
        ];

        for description in test_cases {
            let service = InvoicedService::from_description(description);

            assert_eq!(service, InvoicedService::Unknown(description.to_string()));

            assert_eq!(service.to_description(), description);
        }
    }

    #[test]
    fn edge_cases() {
        let description = " element_boost:123:30 ";
        let service = InvoicedService::from_description(description);
        assert_eq!(service, InvoicedService::Unknown(description.to_string()));

        let description = "element_boost:123_456:30";
        let service = InvoicedService::from_description(description);
        assert_eq!(service, InvoicedService::Unknown(description.to_string()));
    }

    #[test]
    fn boost_creation_and_conversion() {
        let boost = InvoicedService::Boost {
            element_id: 123,
            duration_days: 45,
        };

        assert_eq!(boost.to_description(), "element_boost:123:45");

        let parsed = InvoicedService::from_description(&boost.to_description());
        assert_eq!(parsed, boost);
    }

    #[test]
    fn comment_creation_and_conversion() {
        let comment = InvoicedService::Comment { comment_id: 999 };

        assert_eq!(comment.to_description(), "element_comment:999:publish");

        let parsed = InvoicedService::from_description(&comment.to_description());
        assert_eq!(parsed, comment);
    }

    #[test]
    fn unknown_preservation() {
        let original = "custom:format:with:multiple:parts";
        let unknown = InvoicedService::Unknown(original.to_string());

        assert_eq!(unknown.to_description(), original);

        let parsed = InvoicedService::from_description(&unknown.to_description());
        assert_eq!(parsed, unknown);
    }

    #[test]
    fn clone_and_eq() {
        let boost1 = InvoicedService::Boost {
            element_id: 100,
            duration_days: 30,
        };

        let boost2 = boost1.clone();
        assert_eq!(boost1, boost2);

        let boost3 = InvoicedService::Boost {
            element_id: 100,
            duration_days: 30,
        };
        assert_eq!(boost1, boost3);

        let boost4 = InvoicedService::Boost {
            element_id: 101,
            duration_days: 30,
        };
        assert_ne!(boost1, boost4);

        let comment = InvoicedService::Comment { comment_id: 100 };
        assert_ne!(boost1, comment);

        let unknown = InvoicedService::Unknown("test".to_string());
        let unknown_clone = unknown.clone();
        assert_eq!(unknown, unknown_clone);
    }

    #[test]
    fn debug_format() {
        let boost = InvoicedService::Boost {
            element_id: 123,
            duration_days: 45,
        };

        println!("{:?}", boost);

        let comment = InvoicedService::Comment { comment_id: 999 };
        println!("{:?}", comment);

        let unknown = InvoicedService::Unknown("test".to_string());
        println!("{:?}", unknown);
    }

    #[test]
    fn test_parse_performance() {
        let start = std::time::Instant::now();

        for _ in 0..10000 {
            let _ = InvoicedService::from_description("element_boost:123:30");
            let _ = InvoicedService::from_description("element_comment:456:publish");
            let _ = InvoicedService::from_description("unknown:format");
        }

        let duration = start.elapsed();
        println!("Parsed 30000 descriptions in {:?}", duration);

        assert!(duration.as_millis() < 50);
    }

    #[test]
    fn invoice_integration() {
        #[derive(Debug)]
        struct Invoice {
            description: String,
        }

        impl Invoice {
            fn parse_service(&self) -> InvoicedService {
                InvoicedService::from_description(&self.description)
            }
        }

        let invoice = Invoice {
            description: "element_boost:123:30".to_string(),
        };

        let service = invoice.parse_service();

        match service {
            InvoicedService::Boost {
                element_id,
                duration_days,
            } => {
                assert_eq!(element_id, 123);
                assert_eq!(duration_days, 30);
            }
            _ => panic!("Expected Boost variant"),
        }
    }
}
