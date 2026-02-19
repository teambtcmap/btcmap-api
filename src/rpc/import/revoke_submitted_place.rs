use crate::{
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    id: Option<i64>,
    origin: Option<String>,
    external_id: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub revoked: bool,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let submission = match params.id {
        Some(id) => Some(db::place_submission::queries::select_by_id(id, pool).await?),
        None => {
            db::place_submission::queries::select_by_origin_and_external_id(
                params.origin.unwrap(),
                params.external_id.unwrap(),
                pool,
            )
            .await?
        }
    };

    let Some(submission) = submission else {
        return Err("can't find place with provided id".into());
    };

    let submission = db::place_submission::queries::set_revoked(submission.id, true, pool).await?;

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
        db::place_submission::blocking_queries::InsertArgs,
        db::{self, test::pool},
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
        let submission = db::place_submission::queries::insert(args, &pool).await?;

        assert_eq!(false, submission.revoked);

        let res = super::run(
            super::Params {
                id: None,
                origin: Some(origin.into()),
                external_id: Some(external_id.into()),
            },
            &pool,
        )
        .await?;

        assert_eq!(1, res.id);
        assert_eq!(origin, res.origin);
        assert_eq!(external_id, res.external_id);

        let submission = db::place_submission::queries::select_by_id(res.id, &pool).await?;

        assert_eq!(true, submission.revoked);

        Ok(())
    }
}
