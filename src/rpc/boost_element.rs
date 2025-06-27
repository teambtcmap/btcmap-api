use crate::{
    boost::Boost,
    conf::Conf,
    db::{self, element::schema::Element, user::schema::User},
    discord, Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use rusqlite::Connection;
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

pub async fn run(params: Params, requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let requesting_user_id = requesting_user.id;
    let element = pool
        .get()
        .await?
        .interact(move |conn| _boost(requesting_user_id, &params.id, params.days, conn))
        .await??;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} boosted element {} ({}) for {} days",
            requesting_user.name,
            element.name(),
            element.id,
            params.days
        ),
    )
    .await;
    Ok(Res {
        id: element.id,
        tags: element.tags,
    })
}

fn _boost(admin_id: i64, id_or_osm_id: &str, days: i64, conn: &Connection) -> Result<Element> {
    let element = db::element::queries::select_by_id_or_osm_id(id_or_osm_id, conn)?;
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
        conn,
    )?;
    Boost::insert(admin_id, element.id, days, conn)?;
    Ok(element)
}
