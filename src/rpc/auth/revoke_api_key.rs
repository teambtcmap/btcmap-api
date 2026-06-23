use crate::db;
use crate::db::main::access_token::schema::AccessToken;
use crate::db::main::user::schema::User;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub label: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
}

impl From<AccessToken> for Res {
    fn from(val: AccessToken) -> Self {
        Self {
            id: val.id,
            label: val.name,
            revoked_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    let error_mask = "Key revocation failed";
    let token = db::main::access_token::queries::select_by_id(params.id, pool)
        .await
        .map_err(|_| error_mask)?;
    if token.user_id != user.id {
        return Err(error_mask.into());
    }
    if token.deleted_at.is_some() {
        return Ok(token.into());
    }
    let token = db::main::access_token::queries::set_deleted_at(
        params.id,
        Some(OffsetDateTime::now_utc()),
        pool,
    )
    .await
    .map_err(|_| error_mask)?;
    Ok(token.into())
}

#[cfg(test)]
mod test {
    use crate::db::main::access_token::{queries, schema::AccessToken};
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::{Role, User};
    use crate::Result;
    use time::OffsetDateTime;

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
    fn from_access_token_marks_revoked_when_deleted_at_present() -> Result<()> {
        let token = AccessToken {
            id: 5,
            user_id: 1,
            name: Some("laptop".into()),
            secret: "secret".into(),
            roles: vec![],
            import_origins: vec![],
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            deleted_at: Some(OffsetDateTime::UNIX_EPOCH),
        };
        let res: super::Res = token.into();
        assert_eq!(res.id, 5);
        assert_eq!(res.label.as_deref(), Some("laptop"));
        assert!(res.revoked_at.is_some());
        Ok(())
    }

    #[test]
    fn from_access_token_marks_not_revoked_when_deleted_at_absent() -> Result<()> {
        let token = AccessToken {
            id: 5,
            user_id: 1,
            name: Some("laptop".into()),
            secret: "secret".into(),
            roles: vec![],
            import_origins: vec![],
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            deleted_at: None,
        };
        let res: super::Res = token.into();
        assert_eq!(res.label.as_deref(), Some("laptop"));
        assert!(res.revoked_at.is_none());
        Ok(())
    }

    #[test]
    fn run_revokes_token_owned_by_caller() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let token = queries::insert(1, "laptop".into(), "secret".into(), vec![], &pool).await?;

            let res = super::run(super::Params { id: token.id }, &user(1), &pool).await?;
            assert_eq!(res.id, token.id);
            assert_eq!(res.label.as_deref(), Some("laptop"));
            assert!(res.revoked_at.is_some());

            let stored = queries::select_by_id(token.id, &pool).await?;
            assert!(stored.deleted_at.is_some());
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn run_is_idempotent_when_called_twice() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let token = queries::insert(1, "laptop".into(), "secret".into(), vec![], &pool).await?;

            let first = super::run(super::Params { id: token.id }, &user(1), &pool).await?;
            let first_ts = first.revoked_at.unwrap();

            let second = super::run(super::Params { id: token.id }, &user(1), &pool).await?;
            assert_eq!(second.id, token.id);
            assert_eq!(second.revoked_at.unwrap(), first_ts);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn run_rejects_token_owned_by_another_user() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let token = queries::insert(1, "laptop".into(), "secret".into(), vec![], &pool).await?;

            let err = match super::run(super::Params { id: token.id }, &user(2), &pool).await {
                Ok(_) => panic!("expected masked error"),
                Err(err) => err,
            };
            assert_eq!(err.to_string(), "Key revocation failed");

            let stored = queries::select_by_id(token.id, &pool).await?;
            assert!(stored.deleted_at.is_none());
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn run_returns_masked_error_for_missing_token() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let err = match super::run(super::Params { id: 999_999 }, &user(1), &pool).await {
                Ok(_) => panic!("expected masked error"),
                Err(err) => err,
            };
            assert_eq!(err.to_string(), "Key revocation failed");
            Ok::<(), crate::Error>(())
        })
    }
}
