use crate::{admin, area::Area, discord, element::Element, report::Report, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashMap, sync::Arc};
use time::{format_description::well_known::Iso8601, OffsetDateTime};
use tracing::info;

const NAME: &str = "generate_reports";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub time_s: f64,
    pub new_reports: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let started_at = OffsetDateTime::now_utc();
    let res = pool.get().await?.interact(generate_reports).await??;
    let time_s = (OffsetDateTime::now_utc() - started_at).as_seconds_f64();
    if res > 0 {
        let log_message = format!(
            "{} generated {} daily reports in {} seconds",
            admin.name, res, time_s,
        );
        info!(log_message);
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    }
    Ok(Res {
        started_at: OffsetDateTime::now_utc(),
        finished_at: OffsetDateTime::now_utc(),
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
        new_reports: res as i64,
    })
}

pub fn generate_reports(conn: &mut Connection) -> Result<usize> {
    let today = OffsetDateTime::now_utc().date();
    info!(date = ?today, "Generating report");
    let today_reports = Report::select_by_date(&today, None, conn)?;
    if !today_reports.is_empty() {
        info!("Found existing reports for today, aborting");
        return Ok(0);
    }
    let all_areas = Area::select_all_except_deleted(conn)?;
    let all_elements = Element::select_all_except_deleted(conn)?;
    let mut reports: HashMap<Area, Map<String, Value>> = HashMap::new();
    for area in all_areas {
        let area_elements = crate::element::service::filter_by_area(&all_elements, &area)?;
        if let Some(report) = generate_new_report_if_necessary(&area, area_elements, conn)? {
            reports.insert(area, report);
        }
    }
    let sp = conn.savepoint()?;
    for (area, report) in &reports {
        insert_report(area.id, report, &sp)?;
    }
    sp.commit()?;
    Ok(reports.len())
}

fn generate_new_report_if_necessary(
    area: &Area,
    area_elements: Vec<Element>,
    conn: &Connection,
) -> Result<Option<Map<String, Value>>> {
    let new_report_tags = generate_report_tags(&area_elements)?;
    let prev_report = Report::select_latest_by_area_id(area.id, conn)?;
    Ok(match prev_report {
        None => Some(new_report_tags),
        Some(latest_report) => {
            if new_report_tags != latest_report.tags {
                Some(new_report_tags)
            } else {
                None
            }
        }
    })
}

fn generate_report_tags(elements: &[Element]) -> Result<Map<String, Value>> {
    info!("Generating report tags");

    let atms: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.tag("amenity") == "atm")
        .collect();

    let onchain_elements: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.tag("payment:onchain") == "yes")
        .collect();

    let lightning_elements: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.tag("payment:lightning") == "yes")
        .collect();

    let lightning_contactless_elements: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.tag("payment:lightning_contactless") == "yes")
        .collect();

    let legacy_elements: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.tag("payment:bitcoin") == "yes")
        .collect();

    let up_to_date_elements: Vec<_> = elements
        .iter()
        .filter(|it| it.overpass_data.up_to_date())
        .collect();

    let outdated_elements: Vec<_> = elements
        .iter()
        .filter(|it| !it.overpass_data.up_to_date())
        .collect();

    let up_to_date_percent: f64 = up_to_date_elements.len() as f64 / elements.len() as f64 * 100.0;
    let up_to_date_percent: i64 = up_to_date_percent as i64;

    let mut tags: Map<String, Value> = Map::new();
    tags.insert("total_elements".into(), elements.len().into());
    tags.insert("total_atms".into(), atms.len().into());
    tags.insert(
        "total_elements_onchain".into(),
        onchain_elements.len().into(),
    );
    tags.insert(
        "total_elements_lightning".into(),
        lightning_elements.len().into(),
    );
    tags.insert(
        "total_elements_lightning_contactless".into(),
        lightning_contactless_elements.len().into(),
    );
    tags.insert(
        "up_to_date_elements".into(),
        up_to_date_elements.len().into(),
    );
    tags.insert("outdated_elements".into(), outdated_elements.len().into());
    tags.insert("legacy_elements".into(), legacy_elements.len().into());
    tags.insert(
        "up_to_date_percent".into(),
        (up_to_date_percent as usize).into(),
    );

    let verification_dates: Vec<i64> = elements
        .iter()
        .filter_map(|it| {
            it.overpass_data
                .verification_date()
                .map(|it| it.unix_timestamp())
        })
        .collect();

    let now = OffsetDateTime::now_utc();
    let verification_dates: Vec<i64> = verification_dates
        .into_iter()
        .filter(|it| *it <= now.unix_timestamp())
        .collect();

    if !verification_dates.is_empty() {
        let avg_verification_date: f64 =
            verification_dates.iter().sum::<i64>() as f64 / verification_dates.len() as f64;
        let avg_verification_date: i64 = avg_verification_date as i64;
        let avg_verification_date = OffsetDateTime::from_unix_timestamp(avg_verification_date);

        if let Ok(avg_verification_date) = avg_verification_date {
            tags.insert(
                "avg_verification_date".into(),
                avg_verification_date.format(&Iso8601::DEFAULT)?.into(),
            );
        }
    }

    Ok(tags)
}

fn insert_report(area_id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<()> {
    let date = OffsetDateTime::now_utc().date();
    info!(area_id, ?date, ?tags, "Inserting new report");
    Report::insert(area_id, &date, tags, conn)?;
    info!(area_id, ?date, "Inserted new report");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{osm::overpass::OverpassElement, test::mock_conn};
    use actix_web::test;
    use geojson::{Feature, GeoJson};
    use serde_json::{json, Map};
    use std::collections::HashMap;
    use time::{macros::date, Duration};

    #[test]
    async fn insert_report() -> Result<()> {
        let conn = mock_conn();
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), json!("test"));
        Area::insert(
            GeoJson::Feature(Feature::default()),
            area_tags,
            "test",
            &conn,
        )?;
        for _ in 1..100 {
            Report::insert(1, &date!(2023 - 11 - 12), &Map::new(), &conn)?;
        }
        Ok(())
    }

    #[test]
    async fn generate_report_tags() -> Result<()> {
        let element_1 = json!({
          "type": "node",
          "id": 25338659,
          "lat": -33.9491670,
          "lon": 25.5640810,
          "timestamp": "2023-02-25T09:22:38Z",
          "version": 16,
          "changeset": 132997506,
          "user": "BTC Map",
          "uid": 18545877,
          "tags": {
            "addr:city": "Gqeberha",
            "addr:postcode": "6045",
            "addr:street": "4th Avenue",
            "air_conditioning": "yes",
            "branch": "Newton Park",
            "brand": "Pick n Pay",
            "brand:wikidata": "Q7190735",
            "brand:wikipedia": "en:Pick n Pay Stores",
            "check_date": "2023-02-15",
            "check_date:currency:XBT": "2023-02-25",
            "contact:website": "https://www.pnp.co.za/",
            "currency:XBT": "yes",
            "currency:ZAR": "yes",
            "diet:vegetarian": "yes",
            "level": "0",
            "name": "Pick n Pay",
            "payment:credit_cards": "yes",
            "payment:debit_cards": "yes",
            "payment:mastercard": "yes",
            "payment:visa": "yes",
            "phone": "+27 41 365 1268",
            "second_hand": "no",
            "shop": "supermarket",
            "source": "survey",
            "stroller": "yes",
            "wheelchair": "yes"
          }
        });

        let element_1: OverpassElement = serde_json::from_value(element_1)?;

        let element_2 = json!({
          "type": "node",
          "id": 6402700275 as i64,
          "lat": 28.4077730,
          "lon": -106.8668376,
          "timestamp": "2023-03-15T18:24:05Z",
          "version": 4,
          "changeset": 133721045,
          "user": "CENTSOARER",
          "uid": 232801,
          "tags": {
            "addr:city": "Cd. Cuauhtémoc",
            "addr:street": "Avenida Agustín Melgar",
            "brand": "Elektra",
            "brand:wikidata": "Q1142753",
            "brand:wikipedia": "es:Grupo Elektra",
            "check_date:currency:XBT": "2023-03-09",
            "currency:XBT": "yes",
            "name": "Elektra",
            "payment:cash": "yes",
            "payment:debit_cards": "yes",
            "payment:lightning": "yes",
            "payment:lightning_contactless": "no",
            "payment:onchain": "yes",
            "shop": "department_store",
            "website": "https://www.elektra.mx"
          }
        });

        let mut element_2: OverpassElement = serde_json::from_value(element_2)?;

        let today = OffsetDateTime::now_utc().date();
        let today_plus_year = today
            .checked_add(Duration::days(356))
            .ok_or("Date overflow")?;
        element_2.tags.as_mut().ok_or("No tags")?.insert(
            "check_date:currency:XBT".into(),
            today_plus_year.to_string().into(),
        );

        let element_1 = Element {
            id: 1,
            overpass_data: element_1,
            tags: HashMap::new(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            deleted_at: None,
        };
        let element_2 = Element {
            id: 2,
            overpass_data: element_2,
            tags: HashMap::new(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            deleted_at: None,
        };
        let report_tags = super::generate_report_tags(&vec![element_1, element_2])?;

        assert_eq!(
            2,
            report_tags["total_elements"]
                .as_i64()
                .ok_or("Not a number")?
        );
        assert_eq!(
            "2023-02-25T00:00:00.000000000Z",
            report_tags["avg_verification_date"]
                .as_str()
                .ok_or("Not a string")?,
        );

        Ok(())
    }
}
