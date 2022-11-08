use crate::model::report;
use rusqlite::named_params;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Sub;
use time::Duration;
use time::OffsetDateTime;

static OVERPASS_API_URL: &str = "https://maps.mail.ru/osm/tools/overpass/api/interpreter";

static OVERPASS_API_QUERY: &str = r#"
    [out:json][timeout:300];
    (
    nwr["currency:XBT"="yes"];
    nwr["payment:bitcoin"="yes"];
    );
    out meta geom;
"#;

pub async fn generate_report(db_conn: Connection) {
    let today = OffsetDateTime::now_utc().date();
    log::info!("Generating report for {today}");

    let existing_report = db_conn.query_row(
        report::SELECT_BY_AREA_ID_AND_DATE,
        named_params![
            ":area_id": "",
            ":date": today.to_string()
        ],
        report::SELECT_BY_AREA_ID_AND_DATE_MAPPER,
    );

    if existing_report.is_ok() {
        log::info!("Found existing report, aborting");
        return;
    }

    log::info!("Querying OSM API, it could take a while...");

    let response = match reqwest::Client::new()
        .post(OVERPASS_API_URL)
        .body(OVERPASS_API_QUERY)
        .send()
        .await
    {
        Ok(ok) => ok,
        Err(err) => {
            log::error!("Failed to fetch response: {err}");
            return;
        }
    };

    log::info!("Fetched new data, response code: {}", response.status());

    let response = match response.json::<Value>().await {
        Ok(ok) => ok,
        Err(err) => {
            log::error!("Failed to read response body: {err}");
            return;
        }
    };

    let elements: &Vec<Value> = match response["elements"].as_array() {
        Some(some) => some,
        None => {
            log::error!("Failed to parse elements");
            return;
        }
    };

    if elements.len() == 0 {
        log::error!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }

    log::info!("Fetched {} elements", elements.len());

    if elements.len() < 5000 {
        log::error!("Data set is most likely invalid, aborting report generation");
        return;
    }

    let onchain_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:onchain"].as_str() == Some("yes"))
        .collect();

    let lightning_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:lightning"].as_str() == Some("yes"))
        .collect();

    let lightning_contactless_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"]["payment:lightning_contactless"].as_str() == Some("yes"))
        .collect();

    let legacy_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| it["tags"].get("payment:bitcoin").is_some())
        .collect();

    let year_ago = today.sub(Duration::days(365));
    log::info!("Today: {today}, year ago: {year_ago}");

    let up_to_date_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| {
            (it["tags"].get("survey:date").is_some()
                && it["tags"]["survey:date"].as_str().unwrap().to_string() > year_ago.to_string())
                || (it["tags"].get("check_date").is_some()
                    && it["tags"]["check_date"].as_str().unwrap().to_string()
                        > year_ago.to_string())
        })
        .collect();

    let outdated_elements: Vec<&Value> = elements
        .iter()
        .filter(|it| {
            (it["tags"].get("check_date").is_none()
                && (it["tags"].get("survey:date").is_none()
                    || (it["tags"].get("survey:date").is_some()
                        && it["tags"]["survey:date"].as_str().unwrap().to_string()
                            <= year_ago.to_string())))
                || (it["tags"].get("survey:date").is_none()
                    && (it["tags"].get("check_date").is_none()
                        || (it["tags"].get("check_date").is_some()
                            && it["tags"]["check_date"].as_str().unwrap().to_string()
                                <= year_ago.to_string())))
        })
        .collect();

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
    let tags: Value = serde_json::to_value(tags).unwrap();

    log::info!("Inserting new report");
    log::info!("{}", serde_json::to_string_pretty(&tags).unwrap());

    db_conn
        .execute(
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : today.to_string(),
                ":tags" : serde_json::to_string(&tags).unwrap(),
            },
        )
        .unwrap();

    log::info!("Finished generating report");
}
