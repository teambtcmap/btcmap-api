use rusqlite::Row;
use serde_json::Value;
use std::str::FromStr;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "user";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Name,
    Password,
    Roles,
    SavedPlaces,
    SavedAreas,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[allow(dead_code)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub password: String,
    pub roles: Vec<Role>,
    pub saved_places: Vec<i64>,
    pub saved_areas: Vec<i64>,
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
                Columns::SavedPlaces,
                Columns::SavedAreas,
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
            Ok(User {
                id: row.get(Columns::Id.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                password: row.get(Columns::Password.as_ref())?,
                roles: Self::parse_roles(row.get(Columns::Roles.as_ref())?),
                saved_places: Self::parse_saved_items(row.get(Columns::SavedPlaces.as_ref())?),
                saved_areas: Self::parse_saved_items(row.get(Columns::SavedAreas.as_ref())?),
                created_at: row.get(Columns::CreatedAt.as_ref())?,
                updated_at: row.get(Columns::UpdatedAt.as_ref())?,
                deleted_at: row.get(Columns::DeletedAt.as_ref())?,
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

    fn parse_saved_items(column_value: String) -> Vec<i64> {
        column_value
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    s.parse().ok()
                }
            })
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
