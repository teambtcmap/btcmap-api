use rusqlite::Connection;
use serde_json::Value;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};

use crate::{element::Element, Result};

pub fn run(osm_type: &str, osm_id: i64, days: i64, conn: &Connection) -> Result<()> {
    println!("Boosting element {}:{} for {} days", osm_type, osm_id, days);

    let element = Element::select_by_osm_type_and_id(osm_type, osm_id, conn)?;

    match element {
        Some(element) => {
            println!("Found element {}:{}", osm_type, osm_id);

            let boost_expires = element.tag("boost:expires");
            println!("Existing boost expires on {}", boost_expires);

            let boost_expires = match boost_expires {
                Value::String(v) => {
                    OffsetDateTime::parse(v, &Iso8601::DEFAULT).unwrap_or(OffsetDateTime::now_utc())
                }
                _ => OffsetDateTime::now_utc(),
            };

            let boost_expires = boost_expires.checked_add(Duration::days(days)).unwrap();
            println!(
                "New boost expires on {}",
                boost_expires.format(&Iso8601::DEFAULT)?
            );

            Element::set_tag(
                element.id,
                "boost:expires",
                &Value::String(boost_expires.format(&Iso8601::DEFAULT)?),
                &conn,
            )?;
        }
        None => {
            eprintln!("Can't find element {}:{}", osm_type, osm_id);
        }
    }

    Ok(())
}
