use crate::db;
use crate::db::main::access_token::schema::AccessTokenInfo;
use crate::db::main::user::schema::User;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub label: Option<String>,
    pub roles: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<AccessTokenInfo> for Res {
    fn from(val: AccessTokenInfo) -> Self {
        let roles: Vec<String> = val.roles.iter().map(|it| it.to_string()).collect();
        Self {
            id: val.id,
            label: val.label,
            roles,
            created_at: val.created_at,
            updated_at: val.updated_at,
        }
    }
}

pub async fn run(user: &User, pool: &Pool) -> Result<Vec<Res>> {
    let tokens = db::main::access_token::queries::select_by_user_id(user.id, pool).await?;
    Ok(tokens.into_iter().map(Into::into).collect())
}

#[cfg(test)]
mod test {
    use crate::db::main::access_token::{queries, schema::AccessTokenInfo};
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::{Role, User};
    use crate::Result;
    use time::OffsetDateTime;

    fn info(id: i64, label: Option<&str>, roles: Vec<Role>) -> AccessTokenInfo {
        AccessTokenInfo {
            id,
            user_id: 1,
            label: label.map(|s| s.to_string()),
            roles,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn user(id: i64) -> User {
        User {
            id,
            name: format!("user_{id}"),
            password: String::new(),
            roles: vec![Role::User],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    #[test]
    fn from_access_token_info() -> Result<()> {
        let res: super::Res = info(42, Some("my laptop"), vec![Role::User]).into();
        assert_eq!(res.id, 42);
        assert_eq!(res.label.as_deref(), Some("my laptop"));
        assert_eq!(res.roles, vec!["user".to_string()]);
        Ok(())
    }

    #[test]
    fn from_access_token_info_without_name() -> Result<()> {
        let res: super::Res = info(7, None, vec![]).into();
        assert_eq!(res.label, None);
        assert!(res.roles.is_empty());
        Ok(())
    }

    #[test]
    fn from_access_token_info_with_admin_roles() -> Result<()> {
        let res: super::Res = info(99, Some("ops"), vec![Role::Admin, Role::User]).into();
        assert_eq!(res.roles, vec!["admin".to_string(), "user".to_string()]);
        Ok(())
    }

    #[test]
    fn run_returns_empty_list_when_user_has_no_tokens() -> Result<()> {
        let tokens = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let pool = pool();
                super::run(&user(42), &pool).await
            })?;
        assert!(tokens.is_empty());
        Ok(())
    }

    #[test]
    fn run_returns_only_tokens_owned_by_the_caller() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            queries::insert(
                1,
                "laptop".into(),
                "secret_1".into(),
                vec![Role::User],
                &pool,
            )
            .await?;
            queries::insert(
                1,
                "phone".into(),
                "secret_2".into(),
                vec![Role::Admin],
                &pool,
            )
            .await?;
            queries::insert(
                2,
                "other_user".into(),
                "secret_3".into(),
                vec![Role::User],
                &pool,
            )
            .await?;

            let tokens = super::run(&user(1), &pool).await?;
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].label.as_deref(), Some("laptop"));
            assert_eq!(tokens[1].label.as_deref(), Some("phone"));
            assert_eq!(tokens[0].roles, vec!["user".to_string()]);
            assert_eq!(tokens[1].roles, vec!["admin".to_string()]);

            let other_tokens = super::run(&user(2), &pool).await?;
            assert_eq!(other_tokens.len(), 1);
            assert_eq!(other_tokens[0].label.as_deref(), Some("other_user"));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn run_excludes_soft_deleted_tokens() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let kept = queries::insert(1, "kept".into(), "s1".into(), vec![], &pool).await?;
            let revoked = queries::insert(1, "revoked".into(), "s2".into(), vec![], &pool).await?;
            queries::set_deleted_at(revoked.id, Some(OffsetDateTime::now_utc()), &pool).await?;

            let tokens = super::run(&user(1), &pool).await?;
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].id, kept.id);
            assert_eq!(tokens[0].label.as_deref(), Some("kept"));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn run_response_does_not_leak_secrets() -> Result<()> {
        let tokens = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let pool = pool();
                queries::insert(
                    1,
                    "laptop".into(),
                    "super-secret".into(),
                    vec![Role::User],
                    &pool,
                )
                .await?;
                super::run(&user(1), &pool).await
            })?;
        let obj = serde_json::to_value(&tokens)?
            .as_array()
            .unwrap()
            .first()
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("label"));
        assert!(obj.contains_key("roles"));
        assert!(!obj.contains_key("secret"), "secret must never be returned");
        assert!(
            !obj.contains_key("user_id"),
            "user_id must never be returned"
        );
        Ok(())
    }
}
