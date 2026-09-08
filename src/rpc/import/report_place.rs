use crate::{
    db::main::place_report::blocking_queries::InsertArgs,
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Params {
    pub origin: String,
    pub place_id: i64,
    pub r#type: String,
    pub extra_fields: Option<JsonObject>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub origin: String,
    pub place_id: i64,
    pub r#type: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let extra_fields = params.extra_fields.unwrap_or_default();

    let origin =
        db::main::place_import_origin::queries::select_by_name(params.origin.clone(), pool)
            .await?
            .ok_or_else(|| format!("import origin '{}' is not configured", params.origin))?;

    let args = InsertArgs {
        place_id: params.place_id,
        origin_id: origin.id,
        r#type: params.r#type,
        extra_fields,
        ticket_url: None,
    };
    let new_report = db::main::place_report::queries::insert(args, pool).await?;
    Ok(Res {
        id: new_report.id,
        origin: origin.name,
        place_id: new_report.place_id,
        r#type: new_report.r#type,
    })
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, main::test::pool},
        Result,
    };
    use actix_web::test;
    use serde_json::{Map, Value};

    #[test]
    async fn report_place() -> Result<()> {
        let pool = pool();

        let params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };

        let res = super::run(params.clone(), &pool).await?;

        assert_eq!(params.origin, res.origin);
        assert_eq!(params.place_id, res.place_id);
        assert_eq!(params.r#type, res.r#type);

        let report = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        assert!(report.ticket_url.is_none());
        assert!(report.extra_fields.is_empty());
        assert_eq!(1, report.origin_id);

        Ok(())
    }

    #[test]
    async fn report_place_stores_extra_fields() -> Result<()> {
        let pool = pool();

        let mut extra = Map::new();
        extra.insert("comment".into(), Value::String("had lunch there".into()));

        let params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: Some(extra.clone()),
        };

        let res = super::run(params.clone(), &pool).await?;
        let report = db::main::place_report::queries::select_by_id(res.id, &pool).await?;
        assert_eq!(extra, report.extra_fields);
        Ok(())
    }

    #[test]
    async fn report_place_creates_a_new_row_on_every_call() -> Result<()> {
        let pool = pool();

        let params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };

        let first = super::run(params.clone(), &pool).await?;
        let second = super::run(params.clone(), &pool).await?;
        let third = super::run(params, &pool).await?;

        assert_ne!(first.id, second.id);
        assert_ne!(second.id, third.id);
        assert_ne!(first.id, third.id);
        Ok(())
    }

    #[test]
    async fn report_place_allows_same_place_id_for_different_origins() -> Result<()> {
        let pool = pool();

        let square_params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };
        let coinos_params = super::Params {
            origin: "coinos".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };

        let square_res = super::run(square_params, &pool).await?;
        let coinos_res = super::run(coinos_params, &pool).await?;

        assert_ne!(square_res.id, coinos_res.id);
        assert_eq!(42, square_res.place_id);
        assert_eq!(42, coinos_res.place_id);

        Ok(())
    }

    #[test]
    async fn report_place_allows_same_origin_for_different_types() -> Result<()> {
        let pool = pool();

        let outdated_params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };
        let missing_params = super::Params {
            origin: "square".into(),
            place_id: 42,
            r#type: "missing_payment_method".into(),
            extra_fields: None,
        };

        let outdated_res = super::run(outdated_params, &pool).await?;
        let missing_res = super::run(missing_params, &pool).await?;

        assert_ne!(outdated_res.id, missing_res.id);
        assert_eq!("verification", outdated_res.r#type);
        assert_eq!("missing_payment_method", missing_res.r#type);

        Ok(())
    }

    #[test]
    async fn report_place_unknown_origin_rejected() -> Result<()> {
        let pool = pool();

        let params = super::Params {
            origin: "not-configured".into(),
            place_id: 42,
            r#type: "verification".into(),
            extra_fields: None,
        };

        let res = super::run(params, &pool).await;
        assert!(res.is_err());
        Ok(())
    }
}
