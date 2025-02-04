use crate::{admin, boost::Boost, conf::Conf, discord, element::Element, Result};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};
use tracing::info;

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
    pub days: i64,
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Element> {
    let admin = admin::service::check_rpc(params.password, "boost_element", &pool).await?;
    let element = pool
        .get()
        .await?
        .interact(move |conn| _boost(admin.id, &params.id, params.days, conn))
        .await??;
    let log_message = format!(
        "Admin {} boosted element {} ({}) for {} days",
        admin.name,
        element.name(),
        element.id,
        params.days,
    );
    info!(log_message);
    discord::post_message(&conf.discord_webhook_api, log_message).await;
    Ok(element)
}

fn _boost(admin_id: i64, id_or_osm_id: &str, days: i64, conn: &Connection) -> Result<Element> {
    let element = Element::select_by_id_or_osm_id(id_or_osm_id, conn)?
        .ok_or(format!("There is no element with id = {}", id_or_osm_id))?;
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
    let element = Element::set_tag(
        element.id,
        "boost:expires",
        &Value::String(boost_expires.format(&Iso8601::DEFAULT)?),
        conn,
    )?;
    Boost::insert(admin_id, element.id, days, conn)?;
    Ok(element)
}
