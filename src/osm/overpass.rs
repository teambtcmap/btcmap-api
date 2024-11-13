use crate::{Error, Result};
use geo::{coord, Coord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, hash::Hash, hash::Hasher};
use time::{
    format_description::well_known::Iso8601, macros::format_description, Date, OffsetDateTime,
};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OverpassElement {
    pub r#type: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>, // for nodes only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>, // for nodes only
    pub timestamp: Option<String>,
    pub version: Option<i64>,
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub uid: Option<i64>,
    pub tags: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>, // for ways and relations only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Value>, // for ways only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Value>, // for ways only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Value>, // for relations only
}

impl PartialEq for OverpassElement {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type && self.id == other.id
    }
}

impl Eq for OverpassElement {}

impl Hash for OverpassElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.r#type.hash(state);
        self.id.hash(state);
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Bounds {
    pub minlon: f64,
    pub maxlon: f64,
    pub minlat: f64,
    pub maxlat: f64,
}

impl OverpassElement {
    pub fn btcmap_id(&self) -> String {
        format!("{}:{}", self.r#type, self.id)
    }

    pub fn up_to_date(&self) -> bool {
        match self.days_since_verified() {
            Some(days) => days < 365,
            None => false,
        }
    }

    pub fn days_since_verified(&self) -> Option<i64> {
        self.verification_date()
            .map(|it| (OffsetDateTime::now_utc() - it).whole_days())
    }

    pub fn verification_date(&self) -> Option<OffsetDateTime> {
        let survey_date = self.tag("survey:date");
        let check_date = self.tag("check_date");
        let bitcoin_check_date = self.tag("check_date:currency:XBT");
        let source_date = self.tag("source:date");

        let mut most_recent_date = "";
        let format = format_description!("[year]-[month]-[day]");

        if Date::parse(survey_date, format).is_ok() && survey_date > most_recent_date {
            most_recent_date = survey_date;
        }

        if Date::parse(check_date, format).is_ok() && check_date > most_recent_date {
            most_recent_date = check_date;
        }

        if Date::parse(bitcoin_check_date, format).is_ok() && bitcoin_check_date > most_recent_date
        {
            most_recent_date = bitcoin_check_date;
        }

        if Date::parse(source_date, format).is_ok() && source_date > most_recent_date {
            most_recent_date = source_date
        }

        OffsetDateTime::parse(
            &format!("{}T00:00:00Z", most_recent_date),
            &Iso8601::DEFAULT,
        )
        .ok()
    }

    pub fn coord(&self) -> Coord {
        match self.r#type.as_str() {
            "node" => coord! { x: self.lon.unwrap(), y: self.lat.unwrap() },
            _ => {
                let bounds = self.bounds.as_ref().unwrap();
                coord! { x: (bounds.minlon + bounds.maxlon) / 2.0, y: (bounds.minlat + bounds.maxlat) / 2.0 }
            }
        }
    }

    pub fn tag(&self, name: &str) -> &str {
        match &self.tags {
            Some(tags) => tags.get(name).map(|it| it.as_str()).unwrap_or(""),
            None => "",
        }
    }

    #[cfg(test)]
    pub fn mock(id: i64) -> OverpassElement {
        OverpassElement {
            r#type: "node".into(),
            id,
            lat: Some(0.0),
            lon: Some(0.0),
            timestamp: Some("".into()),
            version: Some(1),
            changeset: Some(1),
            user: Some("".into()),
            uid: Some(1),
            tags: Some(HashMap::new()),
            bounds: None,
            nodes: None,
            geometry: None,
            members: None,
        }
    }
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

    if response.elements.is_empty() {
        Err(Error::OverpassApi(format!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response)?
        )))?
    }

    info!(elements = response.elements.len(), "Fetched elements");

    if response.elements.len() < 5000 {
        Err(Error::OverpassApi("Data set is most likely invalid".into()))?
    }

    Ok(response.elements)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::OverpassElement;

    #[test]
    fn get_tag_value() {
        let mut tags = HashMap::new();
        tags.insert("foo".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("bar", element.tag("foo"));
        assert_eq!("", element.tag("missing"));
    }
}
