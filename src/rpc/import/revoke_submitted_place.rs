use crate::{
    db::main::{access_token::schema::AccessToken, user::schema::Role},
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub id: Option<i64>,
    pub origin: Option<String>,
    pub external_id: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub revoked: bool,
}

pub async fn run(params: Params, roles: &[Role], token: &AccessToken, pool: &Pool) -> Result<Res> {
    let submission = match params.id {
        Some(id) => Some(db::main::place_submission::queries::select_by_id(id, pool).await?),
        None => {
            let Some(origin) = params.origin else {
                return Err("missing parameter: origin or id".into());
            };
            let Some(external_id) = params.external_id else {
                return Err("missing parameter: external_id or id".into());
            };
            db::main::place_submission::queries::select_by_origin_and_external_id(
                origin,
                external_id,
                pool,
            )
            .await?
        }
    };

    let Some(submission) = submission else {
        return Err("can't find place with provided id".into());
    };

    super::ensure_can_access_origin(roles, token, &submission.origin)?;

    let submission =
        db::main::place_submission::queries::set_revoked(submission.id, true, pool).await?;

    Ok(Res {
        id: submission.id,
        origin: submission.origin,
        external_id: submission.external_id,
        revoked: submission.revoked,
    })
}

#[cfg(test)]
mod test {
    use crate::{
        db::main::access_token::schema::AccessToken,
        db::main::place_submission::blocking_queries::InsertArgs,
        db::main::user::schema::Role,
        db::{self, main::test::pool},
        Result,
    };
    use actix_web::test;
    use geojson::JsonObject;

    #[test]
    async fn submit_place() -> Result<()> {
        let origin = "foo";
        let external_id = "bar";
        let lat = 1.23;
        let lon = 4.56;
        let category = "foobar";
        let name = "name";
        let extra_fields = JsonObject::new();

        let pool = pool();

        let args = InsertArgs {
            origin: origin.into(),
            external_id: external_id.into(),
            lat,
            lon,
            category: category.into(),
            name: name.into(),
            extra_fields,
        };
        let submission = db::main::place_submission::queries::insert(args, &pool).await?;

        assert!(!submission.revoked);

        let admin_token = AccessToken {
            id: 1,
            user_id: 1,
            name: None,
            secret: "secret".to_string(),
            roles: vec![Role::Admin],
            import_origins: vec![],
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
            deleted_at: None,
        };

        let res = super::run(
            super::Params {
                id: None,
                origin: Some(origin.into()),
                external_id: Some(external_id.into()),
            },
            &[Role::Admin],
            &admin_token,
            &pool,
        )
        .await?;

        assert_eq!(1, res.id);
        assert_eq!(origin, res.origin);
        assert_eq!(external_id, res.external_id);

        let submission = db::main::place_submission::queries::select_by_id(res.id, &pool).await?;

        assert!(submission.revoked);

        Ok(())
    }
}
