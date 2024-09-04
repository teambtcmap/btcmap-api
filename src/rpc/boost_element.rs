use crate::{auth::Token, discord, element::Element, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub token: String,
    pub id: String,
    pub days: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| _boost(&args.id, args.days, conn))
        .await??;
    let log_message = format!(
        "{} boosted element {} https://api.btcmap.org/v3/elements/{} for {} days",
        token.owner,
        element.name(),
        element.id,
        args.days,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element)
}

fn _boost(id_or_osm_id: &str, days: i64, conn: &Connection) -> Result<Element> {
    let element = Element::select_by_id_or_osm_id(id_or_osm_id, conn)?.unwrap();
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
        &conn,
    )?;
    Ok(element)
}
