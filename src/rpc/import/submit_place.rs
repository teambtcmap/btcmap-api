use crate::{
    db::place_submission::blocking_queries::InsertArgs,
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Params {
    origin: String,
    external_id: String,
    lat: f64,
    lon: f64,
    category: String,
    name: String,
    extra_fields: Option<JsonObject>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let extra_fields = params.extra_fields.unwrap_or_default();

    let existing_submission = db::place_submission::queries::select_by_origin_and_external_id(
        params.origin.clone(),
        params.external_id.clone(),
        pool,
    )
    .await?;

    match existing_submission {
        Some(mut existing_submission) => {
            if existing_submission.revoked {
                existing_submission =
                    db::place_submission::queries::set_revoked(existing_submission.id, false, pool)
                        .await?;
            }

            let mut fields_changed = false;

            if params.lat != existing_submission.lat {
                fields_changed = true
            }

            if params.lon != existing_submission.lon {
                fields_changed = true
            }

            if params.category != existing_submission.category {
                fields_changed = true
            }

            if params.name != existing_submission.name {
                fields_changed = true
            }

            if extra_fields != existing_submission.extra_fields {
                fields_changed = true
            }

            if fields_changed {
                existing_submission = db::place_submission::queries::set_fields(
                    existing_submission.id,
                    params.lat,
                    params.lon,
                    params.category,
                    params.name,
                    &extra_fields,
                    pool,
                )
                .await?;
            }

            Ok(Res {
                id: existing_submission.id,
                origin: existing_submission.origin,
                external_id: existing_submission.external_id,
            })
        }
        None => {
            let args = InsertArgs {
                origin: params.origin,
                external_id: params.external_id,
                lat: params.lat,
                lon: params.lon,
                category: params.category,
                name: params.name,
                extra_fields,
            };
            let new_submission = db::place_submission::queries::insert(args, pool).await?;
            Ok(Res {
                id: new_submission.id,
                origin: new_submission.origin,
                external_id: new_submission.external_id,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, test::pool},
        Result,
    };
    use actix_web::test;

    #[test]
    async fn submit_place() -> Result<()> {
        let params = super::Params {
            origin: "foo".into(),
            external_id: "bar".into(),
            lat: 1.23,
            lon: 3.45,
            category: "foobar".into(),
            name: "name".into(),
            extra_fields: None,
        };

        let pool = pool();

        let res = super::run(params.clone(), &pool).await?;

        assert_eq!(1, res.id);
        assert_eq!(params.origin, res.origin);
        assert_eq!(params.external_id, res.external_id);

        // handle repeated call

        let new_params = super::Params {
            origin: "foo".into(),
            external_id: "bar".into(),
            lat: 1.23,
            lon: 3.45,
            category: "new_category".into(),
            name: "name".into(),
            extra_fields: None,
        };

        let res = super::run(new_params.clone(), &pool).await?;

        assert_eq!(1, res.id);
        assert_eq!(params.origin, res.origin);
        assert_eq!(params.external_id, res.external_id);

        let submission = db::place_submission::queries::select_by_id(res.id, &pool).await?;

        assert_eq!(new_params.category, submission.category);

        Ok(())
    }
}
