use crate::{
    db::{self, element::schema::Element},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub days: i64,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    tags: JsonObject,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let element = boost(&params.id, params.days, pool).await?;
    Ok(Res {
        id: element.id,
        tags: element.tags,
    })
}

async fn boost(id_or_osm_id: &str, days: i64, pool: &Pool) -> Result<Element> {
    let element = db::element::queries::select_by_id_or_osm_id(id_or_osm_id, pool).await?;
    let boost_expires = element.tag("boost:expires");
    let boost_expires = match boost_expires {
        Value::String(v) => {
            OffsetDateTime::parse(v, &Iso8601::DEFAULT).unwrap_or(OffsetDateTime::now_utc())
        }
        _ => OffsetDateTime::now_utc(),
    };
    let boost_expires = if boost_expires < OffsetDateTime::now_utc() {
        OffsetDateTime::now_utc()
    } else {
        boost_expires
    };
    let boost_expires = boost_expires.checked_add(Duration::days(days)).unwrap();
    let element = db::element::queries::set_tag(
        element.id,
        "boost:expires",
        &Value::String(boost_expires.format(&Iso8601::DEFAULT)?),
        pool,
    )
    .await?;
    Ok(element)
}
