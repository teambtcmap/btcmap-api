use crate::db::main::user::schema::Role;
use rusqlite::Row;
use serde_json::Value;
use std::{str::FromStr, sync::OnceLock};
use time::OffsetDateTime;

pub const TABLE: &str = "access_token";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
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
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            Ok(AccessToken {
                id: row.get(Columns::Id.as_ref())?,
                user_id: row.get(Columns::UserId.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                secret: row.get(Columns::Secret.as_ref())?,
                roles: Self::parse_roles(row.get(Columns::Roles.as_ref())?)?,
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
            })
        }
    }

    fn parse_roles(column_value: Value) -> rusqlite::Result<Vec<Role>> {
        let roles: Vec<String> = serde_json::from_value(column_value)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(roles
            .into_iter()
            .filter_map(|s| Role::from_str(&s).ok())
            .collect())
    }
}
