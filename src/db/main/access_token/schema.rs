use crate::db::user::schema::Role;
use rusqlite::Row;
use serde_json::Value;
use std::{str::FromStr, sync::OnceLock};
use time::OffsetDateTime;

pub const NAME: &str = "access_token";

pub enum Columns {
    Id,
    UserId,
    Name,
    Secret,
    Roles,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::UserId => "user_id",
            Columns::Name => "name",
            Columns::Secret => "secret",
            Columns::Roles => "roles",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug)]
pub struct AccessToken {
    pub id: i64,
    pub user_id: i64,
    pub name: Option<String>,
    pub secret: String,
    pub roles: Vec<Role>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl AccessToken {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::UserId,
                Columns::Name,
                Columns::Secret,
                Columns::Roles,
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

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            Ok(AccessToken {
                id: row.get(Columns::Id.as_str())?,
                user_id: row.get(Columns::UserId.as_str())?,
                name: row.get(Columns::Name.as_str())?,
                secret: row.get(Columns::Secret.as_str())?,
                roles: Self::parse_roles(row.get(Columns::Roles.as_str())?),
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }

    fn parse_roles(column_value: Value) -> Vec<Role> {
        let roles: Vec<String> = serde_json::from_value(column_value).unwrap_or_default();
        roles
            .into_iter()
            .filter_map(|s| Role::from_str(&s).ok())
            .collect()
    }
}
