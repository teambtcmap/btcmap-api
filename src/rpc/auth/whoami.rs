use crate::db::user::schema::User;
use crate::Result;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub roles: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub async fn run(user: &User) -> Result<Res> {
    let roles: Vec<String> = user.roles.iter().map(|it| it.to_string()).collect();
    Ok(Res {
        name: user.name.clone(),
        roles,
        created_at: OffsetDateTime::parse(&user.created_at, &Rfc3339)?,
    })
}

#[cfg(test)]
mod test {
    use crate::db::user::schema::{Role, User};
    use crate::Result;
    use actix_web::test;
    use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

    #[test]
    async fn success() {
        let user = User {
            id: 1,
            name: "Test User".to_string(),
            password: "".to_string(),
            roles: vec![Role::Admin, Role::User],
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        };

        let result = super::run(&user).await.unwrap();

        assert_eq!(result.name, "Test User");
        assert_eq!(result.roles, vec!["admin".to_string(), "user".to_string()]);
        assert_eq!(
            result.created_at,
            OffsetDateTime::parse("2023-01-01T00:00:00Z", &Rfc3339).unwrap()
        );
    }

    #[test]
    async fn empty_name() {
        // Test with empty name (should still work)
        let user = User {
            id: 1,
            name: "".into(),
            password: "".to_string(),
            roles: vec![Role::Admin, Role::User],
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        };

        let result = super::run(&user).await.unwrap();
        assert_eq!(result.name, "");
    }

    #[test]
    async fn empty_roles() {
        // Test with empty roles
        let user = User {
            id: 1,
            name: "".into(),
            password: "".to_string(),
            roles: vec![],
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        };

        let result = super::run(&user).await.unwrap();
        assert!(result.roles.is_empty());
    }

    #[test]
    async fn test_run_invalid_timestamp() {
        // Test with invalid timestamp format
        let user = User {
            id: 1,
            name: "".into(),
            password: "".to_string(),
            roles: vec![],
            created_at: "not-a-timestamp".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        };

        let result = super::run(&user).await;
        assert!(matches!(result, Err(crate::Error::Parse(_))));
    }

    #[test]
    async fn future_date() -> Result<()> {
        // Test with a future date (should still work if format is correct)
        let future_date = OffsetDateTime::now_utc().saturating_add(Duration::days(10_000));
        let user = User {
            id: 1,
            name: "".into(),
            password: "".into(),
            roles: vec![],
            created_at: future_date.format(&Rfc3339)?,
            updated_at: future_date.format(&Rfc3339)?,
            deleted_at: None,
        };

        let result = super::run(&user).await?;
        assert_eq!(result.created_at, future_date);
        Ok(())
    }
}
