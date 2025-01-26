use crate::{admin, boost::Boost, conf::Conf, discord, element::Element, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};
use tracing::info;

const NAME: &str = "boost_element";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub days: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let element = pool
        .get()
        .await?
        .interact(move |conn| _boost(admin.id, &args.id, args.days, conn))
        .await??;
    let log_message = format!(
        "Admin {} boosted element {} https://api.btcmap.org/v3/elements/{} for {} days",
        admin.name,
        element.name(),
        element.id,
        args.days,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
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
