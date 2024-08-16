use crate::area::Area;
use crate::element::Element;
use crate::report::Report;
use crate::Result;
use rusqlite::Connection;
use serde_json::Map;
use serde_json::Value;
use std::collections::HashSet;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tracing::info;

pub fn run(conn: &mut Connection) -> Result<()> {
    let sp = conn.savepoint()?;
    let today = OffsetDateTime::now_utc().date();
    info!(date = ?today, "Generating report");

    let today_reports = Report::select_by_date(&today, None, &sp)?;

    if !today_reports.is_empty() {
        info!("Found existing reports for today, aborting");
        return Ok(());
    }

    let all_elements: Vec<Element> = Element::select_all(None, &sp)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();

    let all_areas: Vec<Area> = Area::select_all(None, &sp)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();

    let mut new_reports = 0;

    for area in all_areas {
        info!(
            area.id,
            area_url_alias = area.tags.get("url_alias").map(|it| it.as_str()),
            "Generating report",
        );

        let area_elements = crate::element::service::filter_by_area(&all_elements, &area)?;

        info!(
            area.id,
            area_url_alias = area.tags.get("url_alias").map(|it| it.as_str()),
            elements = area_elements.len(),
            "Processing area",
        );
        let new_report_tags = generate_report_tags(&area_elements)?;
        let prev_report = Report::select_latest_by_area_id(area.id, &sp)?;

        match prev_report {
            None => {
                info!(area.id, "There is no report history");
                insert_report(area.id, &new_report_tags, &sp)?;
                new_reports = new_reports + 1;
            }
            Some(latest_report) => {
                if &new_report_tags != &latest_report.tags {
                    info!("Tags changed");
                    log_diff(&latest_report.tags, &new_report_tags)?;
                    insert_report(area.id, &new_report_tags, &sp)?;
                    new_reports = new_reports + 1;
                }
            }
        }
    }

    sp.commit()?;
    info!(new_reports);

    Ok(())
}

fn log_diff(map_1: &Map<String, Value>, map_2: &Map<String, Value>) -> Result<()> {
    let mut keys: HashSet<&String> = HashSet::new();

    for key in map_1.keys() {
        keys.insert(key);
    }

    for key in map_2.keys() {
        keys.insert(key);
    }

    for key in keys {
        if map_1.get(key) != map_2.get(key) {
            info!(
                key,
                value_1 = serde_json::to_string(&map_1.get(key))?,
                value_2 = serde_json::to_string(&map_2.get(key))?,
            );
        }
    }

    Ok(())
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

    let now = OffsetDateTime::now_utc();

    let verification_dates: Vec<i64> = elements
        .iter()
        .filter_map(|it| {
            it.overpass_data
                .verification_date()
                .map(|it| it.unix_timestamp())
        })
        .filter_map(|it| {
            if it > now.unix_timestamp() {
                None
            } else {
                Some(it)
            }
        })
        .collect();

    if verification_dates.len() > 0 {
        let avg_verification_date: f64 =
            verification_dates.iter().sum::<i64>() as f64 / verification_dates.len() as f64;
        let avg_verification_date: i64 = avg_verification_date as i64;
        let avg_verification_date = OffsetDateTime::from_unix_timestamp(avg_verification_date);

        if let Ok(avg_verification_date) = avg_verification_date {
            tags.insert(
                "avg_verification_date".into(),
                avg_verification_date
                    .format(&Iso8601::DEFAULT)
                    .unwrap()
                    .into(),
            );
        }
    }

    Ok(tags)
}

fn insert_report(area_id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<()> {
    let date = OffsetDateTime::now_utc().date();
    info!(area_id, ?date, ?tags, "Inserting new report");
    Report::insert(area_id, &date, &tags, conn)?;
    info!(area_id, ?date, "Inserted new report");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{osm::overpass::OverpassElement, test::mock_state};
    use serde_json::{json, Map};
    use std::collections::HashMap;
    use time::{macros::date, Duration};
    use tokio::test;

    #[test]
    async fn insert_report() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), json!("test"));
        Area::insert(&area_tags, &state.conn)?;
        for _ in 1..100 {
            state
                .report_repo
                .insert(1, &date!(2023 - 11 - 12), &Map::new())
                .await?;
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
        let today_plus_year = today.checked_add(Duration::days(356)).unwrap();
        element_2.tags.as_mut().unwrap().insert(
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

        assert_eq!(2, report_tags["total_elements"].as_i64().unwrap());
        assert_eq!(
            "2023-02-25T00:00:00.000000000Z",
            report_tags["avg_verification_date"].as_str().unwrap(),
        );

        Ok(())
    }
}
