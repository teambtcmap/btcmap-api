use crate::command::sync;
use crate::model::area;
use crate::model::report;
use crate::model::Area;
use crate::Error;
use crate::Result;
use geo::coord;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use geojson::GeoJson;
use geojson::Geometry;
use rusqlite::named_params;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Sub;
use time::format_description::well_known::Iso8601;
use time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::error;
use tracing::info;
use tracing::warn;

pub async fn run(mut db: Connection) -> Result<()> {
    let today = OffsetDateTime::now_utc().date();
    info!(date = ?today, "Generating report");

    let existing_report = db.query_row(
        report::SELECT_BY_AREA_ID_AND_DATE,
        named_params![
            ":area_id": "",
            ":date": today.to_string()
        ],
        report::SELECT_BY_AREA_ID_AND_DATE_MAPPER,
    );

    if existing_report.is_ok() {
        info!("Found existing report, aborting");
        return Ok(());
    }

    info!("Querying OSM API, it could take a while...");

    let response = reqwest::Client::new()
        .post(sync::OVERPASS_API_URL)
        .body(sync::OVERPASS_API_QUERY)
        .send()
        .await?;

    info!(response_status_code = ?response.status(), "Fetched new data");

    let response = response.json::<Value>().await?;

    let elements: Vec<Value> = response["elements"]
        .as_array()
        .ok_or(Error::Other("Failed to parse elements".into()))?
        .to_owned();

    if elements.len() == 0 {
        Err(Error::Other(format!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response)?
        )))?
    }

    info!(elements = elements.len(), "Fetched elements");

    if elements.len() < 5000 {
        Err(Error::Other(
            "Data set is most likely invalid, aborting report generation".into(),
        ))?
    }

    let areas: Vec<Area> = db
        .prepare(area::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            area::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Area>, _>>()?
        .into_iter()
        .filter(|it| it.deleted_at.len() == 0)
        .collect();

    let tx = db.transaction()?;

    let report_tags = generate_report_tags(elements.iter().collect())?;
    insert_report("", report_tags, &tx).await?;

    for area in areas {
        info!(area.id, "Generating report");
        let mut area_elements: Vec<&Value> = vec![];

        if area.tags.contains_key("geo_json") {
            let geo_json = area.tags["geo_json"].to_string();
            let geo_json: Result<GeoJson, _> = geo_json.parse();

            let geo_json = match geo_json {
                Ok(geo_json) => geo_json,
                Err(e) => {
                    error!(?e, "Failed to parse GeoJSON");
                    continue;
                }
            };

            let mut geometries: Vec<&Geometry> = vec![];

            match &geo_json {
                GeoJson::FeatureCollection(v) => {
                    for feature in &v.features {
                        if let Some(v) = &feature.geometry {
                            geometries.push(v);
                        }
                    }
                }
                GeoJson::Feature(v) => {
                    if let Some(v) = &v.geometry {
                        geometries.push(v);
                    }
                }
                GeoJson::Geometry(v) => geometries.push(v),
            };

            for element in &elements {
                for geometry in &geometries {
                    match &geometry.value {
                        geojson::Value::MultiPolygon(_) => {
                            let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                            if multi_poly.contains(&coord! { x: lon(element), y: lat(element) }) {
                                area_elements.push(element);
                            }
                        }
                        geojson::Value::Polygon(_) => {
                            let poly: Polygon = (&geometry.value).try_into().unwrap();

                            if poly.contains(&coord! { x: lon(element), y: lat(element) }) {
                                area_elements.push(element);
                            }
                        }
                        geojson::Value::LineString(_) => {
                            let line_string: LineString = (&geometry.value).try_into().unwrap();

                            if line_string.contains(&coord! { x: lon(element), y: lat(element) }) {
                                area_elements.push(element);
                            }
                        }
                        _ => continue,
                    }
                }
            }
        } else {
            for element in &elements {
                if area.contains(lat(element), lon(element)) {
                    area_elements.push(element)
                }
            }
        }

        info!(area.id, elements = area_elements.len(), "Processing area");
        let report_tags = generate_report_tags(area_elements)?;
        insert_report(&area.id, report_tags, &tx).await?;
    }

    tx.commit()?;

    Ok(())
}

fn generate_report_tags(elements: Vec<&Value>) -> Result<Value> {
    info!("Generating report tags");

    let onchain_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:onchain"].as_str() == Some("yes"))
        .copied()
        .collect();

    let lightning_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:lightning"].as_str() == Some("yes"))
        .copied()
        .collect();

    let lightning_contactless_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:lightning_contactless"].as_str() == Some("yes"))
        .copied()
        .collect();

    let legacy_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:bitcoin"].as_str() == Some("yes"))
        .copied()
        .collect();

    let up_to_date_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| up_to_date(it))
        .copied()
        .collect();

    let outdated_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| !up_to_date(it))
        .copied()
        .collect();

    let up_to_date_percent: f64 = up_to_date_elements.len() as f64 / elements.len() as f64 * 100.0;
    let up_to_date_percent: i64 = up_to_date_percent as i64;

    let grade = match up_to_date_percent {
        95..101 => 5,
        75..95 => 4,
        50..75 => 3,
        25..50 => 2,
        _ => 1,
    };

    let mut tags: HashMap<String, Value> = HashMap::new();
    tags.insert("total_elements".into(), elements.len().into());
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
    tags.insert("grade".into(), grade.into());

    let now = OffsetDateTime::now_utc();

    let verification_dates: Vec<i64> = elements
        .iter()
        .filter_map(|it| verification_date(it).map(|it| it.unix_timestamp()))
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

    let tags: Value = serde_json::to_value(tags)?;

    Ok(tags)
}

async fn insert_report(area_id: &str, tags: Value, db: &Connection) -> Result<()> {
    let date = OffsetDateTime::now_utc().date().to_string();
    info!(area_id, date, ?tags, "Inserting new report");

    for attempt in 1..10 {
        let res = db.execute(
            report::INSERT,
            named_params! {
                ":area_id" : area_id,
                ":date" : date,
                ":tags" : serde_json::to_string(&tags)?,
            },
        );

        match &res {
            Ok(_) => {
                break;
            }
            Err(e) => {
                if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    warn!(
                        area_id,
                        ?date,
                        attempt,
                        "Failed to insert report due to constraint violation",
                    );
                    sleep(tokio::time::Duration::from_millis(10)).await
                } else {
                    res?;
                }
            }
        }
    }

    info!(area_id, ?date, "Inserted new report");
    Ok(())
}

pub fn up_to_date(osm_json: &Value) -> bool {
    let verification_date = verification_date(osm_json)
        .map(|it| it.to_string().to_string())
        .unwrap_or(String::new());
    let year_ago = OffsetDateTime::now_utc().date().sub(Duration::days(365));
    verification_date.as_str() > year_ago.to_string().as_str()
}

pub fn verification_date(osm_json: &Value) -> Option<OffsetDateTime> {
    let tags: &Value = &osm_json["tags"];

    let survey_date = tags["survey:date"].as_str().unwrap_or("");
    let check_date = tags["check_date"].as_str().unwrap_or("");
    let bitcoin_check_date = tags["check_date:currency:XBT"].as_str().unwrap_or("");

    let mut most_recent_date = "";

    if survey_date > most_recent_date {
        most_recent_date = survey_date;
    }

    if check_date > most_recent_date {
        most_recent_date = check_date;
    }

    if bitcoin_check_date > most_recent_date {
        most_recent_date = bitcoin_check_date;
    }

    OffsetDateTime::parse(
        &format!("{}T00:00:00Z", most_recent_date),
        &Iso8601::DEFAULT,
    )
    .ok()
}

pub fn lat(osm_json: &Value) -> f64 {
    match osm_json["type"].as_str().unwrap() {
        "node" => osm_json["lat"].as_f64().unwrap(),
        _ => {
            let min_lat = osm_json["bounds"]["minlat"].as_f64().unwrap();
            let max_lat = osm_json["bounds"]["maxlat"].as_f64().unwrap();
            (min_lat + max_lat) / 2.0
        }
    }
}

pub fn lon(osm_json: &Value) -> f64 {
    match osm_json["type"].as_str().unwrap() {
        "node" => osm_json["lon"].as_f64().unwrap(),
        _ => {
            let min_lon = osm_json["bounds"]["minlon"].as_f64().unwrap();
            let max_lon = osm_json["bounds"]["maxlon"].as_f64().unwrap();
            (min_lon + max_lon) / 2.0
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::command::db;

    use super::*;

    #[actix_web::test]
    async fn insert_report() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        for i in 1..100 {
            super::insert_report(&i.to_string(), Value::Null, &conn).await?;
        }

        Ok(())
    }

    #[test]
    fn generate_report_tags() -> Result<()> {
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

        let mut element_2 = json!({
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

        let today = OffsetDateTime::now_utc().date();
        let today_plus_year = today.checked_add(Duration::days(356)).unwrap();
        element_2["tags"].as_object_mut().unwrap()["check_date:currency:XBT"] =
            today_plus_year.to_string().into();

        let report_tags = super::generate_report_tags(vec![&element_1, &element_2])?;

        assert_eq!(2, report_tags["total_elements"].as_i64().unwrap());
        assert_eq!(
            "2023-02-25T00:00:00.000000000Z",
            report_tags["avg_verification_date"].as_str().unwrap(),
        );

        Ok(())
    }
}
