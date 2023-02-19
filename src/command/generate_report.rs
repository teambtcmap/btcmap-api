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
use time::Date;
use time::Duration;
use time::OffsetDateTime;

pub async fn run(mut db: Connection) -> Result<()> {
    let today = OffsetDateTime::now_utc().date();
    log::info!("Generating report for {today}");

    let existing_report = db.query_row(
        report::SELECT_BY_AREA_ID_AND_DATE,
        named_params![
            ":area_id": "",
            ":date": today.to_string()
        ],
        report::SELECT_BY_AREA_ID_AND_DATE_MAPPER,
    );

    if existing_report.is_ok() {
        log::info!("Found existing report, aborting");
        return Ok(());
    }

    log::info!("Querying OSM API, it could take a while...");

    let response = reqwest::Client::new()
        .post(sync::OVERPASS_API_URL)
        .body(sync::OVERPASS_API_QUERY)
        .send()
        .await?;

    log::info!("Fetched new data, response code: {}", response.status());

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

    log::info!("Fetched {} elements", elements.len());

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

    generate_report("", &today, elements.iter().collect(), &tx)?;

    for area in areas {
        log::info!("Generating report for area {}", area.id);
        let mut area_elements: Vec<&Value> = vec![];

        if area.tags.contains_key("geo_json") {
            let geo_json = area.tags["geo_json"].to_string();
            let geo_json: Result<GeoJson, _> = geo_json.parse();

            let geo_json = match geo_json {
                Ok(geo_json) => geo_json,
                Err(e) => {
                    log::error!("Failed to parse GeoJSON: {}", e);
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

        log::info!("Area: {}, elements: {}", area.id, area_elements.len());
        generate_report(&area.id, &today, area_elements, &tx)?;
    }

    tx.commit()?;

    Ok(())
}

fn generate_report(
    area_id: &str,
    date: &Date,
    elements: Vec<&Value>,
    db: &Connection,
) -> Result<()> {
    log::info!("Generating report for date = {date}, area = {area_id}");

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
        .filter(|it| it["tags"].get("payment:bitcoin").is_some())
        .copied()
        .collect();

    let year_ago = date.sub(Duration::days(365));
    log::info!("Date: {date}, year ago: {year_ago}");

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

    let mut tags: HashMap<&str, usize> = HashMap::new();
    tags.insert("total_elements", elements.len());
    tags.insert("total_elements_onchain", onchain_elements.len());
    tags.insert("total_elements_lightning", lightning_elements.len());
    tags.insert(
        "total_elements_lightning_contactless",
        lightning_contactless_elements.len(),
    );
    tags.insert("up_to_date_elements", up_to_date_elements.len());
    tags.insert("outdated_elements", outdated_elements.len());
    tags.insert("legacy_elements", legacy_elements.len());
    tags.insert("up_to_date_percent", up_to_date_percent as usize);
    tags.insert("grade", grade);
    let tags: Value = serde_json::to_value(tags)?;

    log::info!("Inserting new report");
    log::info!("{}", serde_json::to_string_pretty(&tags)?);

    db.execute(
        report::INSERT,
        named_params! {
            ":area_id" : area_id,
            ":date" : date.to_string(),
            ":tags" : serde_json::to_string(&tags)?,
        },
    )?;

    log::info!("Finished generating report for date = {date}, area = {area_id}");

    Ok(())
}

pub fn up_to_date(osm_json: &Value) -> bool {
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

    let year_ago = OffsetDateTime::now_utc().date().sub(Duration::days(365));

    most_recent_date > year_ago.to_string().as_str()
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
