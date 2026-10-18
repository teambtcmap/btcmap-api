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

use std::fmt;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Role {
    User,
    Admin,
    Root,
    PlacesSource,
    EventManager,
    AreaManager,
    Dashboard,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Admin => write!(f, "admin"),
            Role::Root => write!(f, "root"),
            Role::PlacesSource => write!(f, "places_source"),
            Role::EventManager => write!(f, "event_manager"),
            Role::AreaManager => write!(f, "area_manager"),
            Role::Dashboard => write!(f, "dashboard"),
        }
    }
}

impl User {
    /// Whether the account can log in with a password: the stored value is
    /// an Argon2 hash. Accounts created via Nostr store an empty string, and
    /// admin secrets carried over by migration 62 were never hashed; neither
    /// can be used to log in, so neither counts.
    pub fn has_password(&self) -> bool {
        argon2::PasswordHash::new(&self.password).is_ok()
    }

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
    use super::User;
    use argon2::Argon2;
    use argon2::PasswordHasher;

    fn user_with_password(password: &str) -> User {
        User {
            id: 1,
            name: "user".into(),
            password: password.into(),
            roles: vec![],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence: vec![],
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    #[test]
    fn has_password_for_an_argon2_hash() {
        let hash = Argon2::default().hash_password(b"pw").unwrap().to_string();
        assert!(user_with_password(&hash).has_password());
    }

    // Accounts created via Nostr
    #[test]
    fn no_password_when_empty() {
        assert!(!user_with_password("").has_password());
    }

    // Admin secrets carried over as-is by migration 62 never became hashes:
    // nothing can log in with them
    #[test]
    fn no_password_for_a_value_that_is_not_a_hash() {
        assert!(!user_with_password("legacy-plaintext-secret").has_password());
    }
}
