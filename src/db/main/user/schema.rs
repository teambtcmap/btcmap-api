use rusqlite::Row;
use serde_json::Value;
use std::str::FromStr;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "user";

pub enum Columns {
    Id,
    Name,
    Password,
    Roles,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Name => "name",
            Columns::Password => "password",
            Columns::Roles => "roles",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[allow(dead_code)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub password: String,
    pub roles: Vec<Role>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

use std::fmt;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Role {
    User,
    Admin,
    Root,
    PlacesSource,
    EventManager,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Admin => write!(f, "admin"),
            Role::Root => write!(f, "root"),
            Role::PlacesSource => write!(f, "places_source"),
            Role::EventManager => write!(f, "event_manager"),
        }
    }
}

impl User {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Name,
                Columns::Password,
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
            Ok(User {
                id: row.get(Columns::Id.as_str())?,
                name: row.get(Columns::Name.as_str())?,
                password: row.get(Columns::Password.as_str())?,
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

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Role::User),
            "admin" => Ok(Role::Admin),
            "root" => Ok(Role::Root),
            "places_source" => Ok(Role::PlacesSource),
            "event_manager" => Ok(Role::EventManager),
            _ => Err(format!("'{}' is not a valid Role", s)),
        }
    }
}
