use crate::db;
use crate::db::main::access_token::schema::AccessToken;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

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

pub async fn run(token: &AccessToken, pool: &Pool) -> Result<Res> {
    if token.deleted_at.is_some() {
        return Ok(Res {
            id: token.id,
            label: token.name.clone(),
            revoked_at: token.deleted_at,
        });
    }
    let token = db::main::access_token::queries::set_deleted_at(
        token.id,
        Some(OffsetDateTime::now_utc()),
        pool,
    )
    .await
    .map_err(|_| "Sign-out failed")?;
    Ok(token.into())
}

#[cfg(test)]
mod test {
    use crate::db::main::access_token::{queries, schema::AccessToken};
    use crate::Result;
    use time::OffsetDateTime;

    fn token(id: i64) -> AccessToken {
        AccessToken {
            id,
            user_id: 1,
            name: Some("laptop".into()),
            secret: "secret".into(),
            roles: vec![],
            import_origins: vec![],
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            deleted_at: None,
        }
    }

    #[test]
    fn from_access_token_marks_revoked_when_deleted_at_present() -> Result<()> {
        let mut token = token(5);
        token.deleted_at = Some(OffsetDateTime::UNIX_EPOCH);
        let res: super::Res = token.into();
        assert_eq!(res.id, 5);
        assert_eq!(res.label.as_deref(), Some("laptop"));
        assert!(res.revoked_at.is_some());
        Ok(())
    }

    #[test]
    fn from_access_token_marks_not_revoked_when_deleted_at_absent() -> Result<()> {
        let token = token(5);
        let res: super::Res = token.into();
        assert_eq!(res.id, 5);
        assert_eq!(res.label.as_deref(), Some("laptop"));
        assert!(res.revoked_at.is_none());
        Ok(())
    }

    #[test]
    fn run_revokes_active_token() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = crate::db::main::test::pool();
            let stored =
                queries::insert(1, "laptop".into(), "secret".into(), vec![], &pool).await?;

            let res = super::run(&stored, &pool).await?;
            assert_eq!(res.id, stored.id);
            assert_eq!(res.label.as_deref(), Some("laptop"));
            assert!(res.revoked_at.is_some());

            let refreshed = queries::select_by_id(stored.id, &pool).await?;
            assert!(refreshed.deleted_at.is_some());
            Ok::<(), crate::Error>(())
        })
    }
}
