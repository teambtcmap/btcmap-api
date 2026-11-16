use rusqlite::Row;
use serde::Deserialize;
use serde::Serialize;
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
    Npub,
    Geofence,
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
    pub npub: Option<String>,
    pub geofence: Vec<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// A user's role, sent as its snake_case name, e.g. `places_source`.
#[derive(
    PartialEq,
    Eq,
    Hash,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    ts_rs::TS,
    strum::Display,
    strum::EnumIter,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Role {
    User,
    Admin,
    Root,
    PlacesSource,
    EventManager,
    AreaManager,
    Dashboard,
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
                Columns::Npub,
                Columns::Geofence,
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
                npub: row.get(Columns::Npub.as_ref())?,
                geofence: Self::parse_saved_items(row.get(Columns::Geofence.as_ref())?),
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
            "area_manager" => Ok(Role::AreaManager),
            "dashboard" => Ok(Role::Dashboard),
            _ => Err(format!("'{}' is not a valid Role", s)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Role;
    use serde_json::Value;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn role_serializes_as_its_display_name() {
        // Responses sent role.to_string() before Role was serialized
        // directly; the wire format must not change
        for role in Role::iter() {
            assert_eq!(
                serde_json::to_value(&role).unwrap(),
                Value::String(role.to_string())
            );
        }
    }

    #[test]
    fn role_parses_its_display_name() {
        // The stored roles column holds Display names and is read back with
        // the hand-written FromStr, so the two must agree on every variant
        for role in Role::iter() {
            assert_eq!(Role::from_str(&role.to_string()), Ok(role));
        }
    }
}
