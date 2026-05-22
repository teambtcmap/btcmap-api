pub mod get_submitted_place;
pub mod revoke_submitted_place;
pub mod submit_place;
pub mod sync_submitted_places;

use crate::db::main::{access_token::schema::AccessToken, user::schema::Role};

pub const IMPORT_ORIGIN_WILDCARD: &str = "*";

pub fn can_access_origin(roles: &[Role], token: &AccessToken, origin: &str) -> bool {
    if roles
        .iter()
        .any(|role| matches!(role, Role::Admin | Role::Root))
    {
        return true;
    }

    roles.iter().any(|role| matches!(role, Role::PlacesSource))
        && token.import_origins.iter().any(|allowed_origin| {
            allowed_origin == IMPORT_ORIGIN_WILDCARD || allowed_origin == origin
        })
}

pub fn ensure_can_access_origin(
    roles: &[Role],
    token: &AccessToken,
    origin: &str,
) -> crate::Result<()> {
    if can_access_origin(roles, token, origin) {
        Ok(())
    } else {
        Err(format!("token is not allowed to access import origin '{origin}'").into())
    }
}

#[cfg(test)]
mod test {
    use crate::db::main::access_token::schema::AccessToken;
    use crate::db::main::user::schema::Role;
    use time::OffsetDateTime;

    fn token(roles: Vec<Role>, import_origins: Vec<String>) -> AccessToken {
        AccessToken {
            id: 1,
            user_id: 1,
            name: Some("source".to_string()),
            secret: "secret".to_string(),
            roles,
            import_origins,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            deleted_at: None,
        }
    }

    #[test]
    fn places_source_can_access_scoped_origin() {
        let token = token(vec![Role::PlacesSource], vec!["square".to_string()]);

        assert!(super::can_access_origin(&token.roles, &token, "square"));
        assert!(!super::can_access_origin(&token.roles, &token, "coinos"));
    }

    #[test]
    fn places_source_wildcard_can_access_all_origins() {
        let token = token(
            vec![Role::PlacesSource],
            vec![super::IMPORT_ORIGIN_WILDCARD.to_string()],
        );

        assert!(super::can_access_origin(&token.roles, &token, "square"));
        assert!(super::can_access_origin(&token.roles, &token, "coinos"));
    }

    #[test]
    fn admin_can_access_all_origins_without_scope() {
        let token = token(vec![Role::Admin], vec![]);

        assert!(super::can_access_origin(&token.roles, &token, "square"));
        assert!(super::can_access_origin(&token.roles, &token, "coinos"));
    }
}
