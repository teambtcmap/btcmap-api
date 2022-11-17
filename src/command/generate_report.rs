use crate::command::sync;
use crate::model::report;
use crate::Error;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Sub;
use time::Duration;
use time::OffsetDateTime;

pub async fn run(db: Connection) -> Result<()> {
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

    let elements: &Vec<Value> = response["elements"]
        .as_array()
        .ok_or(Error::Other("Failed to parse elements".into()))?;

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
    let tags: Value = serde_json::to_value(tags)?;

    log::info!("Inserting new report");
    log::info!("{}", serde_json::to_string_pretty(&tags)?);

    db.execute(
        report::INSERT,
        named_params! {
            ":area_id" : "",
            ":date" : today.to_string(),
            ":tags" : serde_json::to_string(&tags)?,
        },
    )?;

    log::info!("Finished generating report");

    Ok(())
}
