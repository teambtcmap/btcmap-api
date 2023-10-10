use crate::{Error, Result, model::OverpassElement};
use serde::{Deserialize, Serialize};
use tracing::info;

static API_URL: &str = "https://overpass-api.de/api/interpreter";

static QUERY: &str = r#"
    [out:json][timeout:300];
    nwr["currency:XBT"=yes];
    out meta geom;
"#;

#[derive(Serialize, Deserialize)]
struct Response {
    version: f64,
    generator: String,
    osm3s: Osm3s,
    elements: Vec<OverpassElement>,
}

#[derive(Serialize, Deserialize)]
struct Osm3s {
    timestamp_osm_base: String,
}

pub async fn query_bitcoin_merchants() -> Result<Vec<OverpassElement>> {
    info!("Querying OSM API, it could take a while...");

    let response = reqwest::Client::new()
        .post(API_URL)
        .body(QUERY)
        .send()
        .await?;

    info!(http_status_code = ?response.status(), "Got OSM API response");

    let response = response.json::<Response>().await?;

    if response.elements.len() == 0 {
        Err(Error::Other(format!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response)?
        )))?
    }

    info!(elements = response.elements.len(), "Fetched elements");

    if response.elements.len() < 5000 {
        Err(Error::Other("Data set is most likely invalid".into()))?
    }

    Ok(response.elements)
}
