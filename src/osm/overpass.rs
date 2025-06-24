use crate::Result;
use geo::{coord, Coord};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{hash::Hash, hash::Hasher};
use time::{
    format_description::well_known::Iso8601, macros::format_description, Date, OffsetDateTime,
};
use tracing::info;

static API_URL: &str = "https://overpass-api.de/api/interpreter";

static QUERY: &str = r#"
    [out:json][timeout:300];
    area["name"="United States"]->.boundaryarea;
    (
      nwr["currency:XBT"=yes];
      way["brand:wikidata"="Q7605233"]["disused:amenity"!~"."](area.boundaryarea);
    );
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
    pub tags: Option<Map<String, Value>>,
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
        self.r#type == other.r#type && self.id == other.id && self.version == other.version
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
            Some(tags) => tags
                .get(name)
                .map(|it| it.as_str().unwrap_or_default())
                .unwrap_or_default(),
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
            tags: Some(Map::new()),
            bounds: None,
            nodes: None,
            geometry: None,
            members: None,
        }
    }
}

#[derive(Serialize)]
pub struct QueryBitcoinMerchantsRes {
    pub elements: Vec<OverpassElement>,
    pub time_s: f64,
}

pub async fn query_bitcoin_merchants() -> Result<QueryBitcoinMerchantsRes> {
    info!("Querying OSM API, it could take a while...");
    let started_at = OffsetDateTime::now_utc();
    let response = reqwest::Client::new()
        .post(API_URL)
        .body(QUERY)
        .send()
        .await?;
    info!(http_status_code = ?response.status(), "Got OSM API response");
    let response = response.json::<Response>().await?;
    if response.elements.is_empty() {
        Err(format!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response)?
        ))?
    }
    info!(elements = response.elements.len(), "Fetched elements");
    if response.elements.len() < 5000 {
        Err("Data set is most likely invalid")?
    }
    Ok(QueryBitcoinMerchantsRes {
        elements: response.elements,
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}

#[cfg(test)]
mod test {
    use super::OverpassElement;
    use serde_json::Map;

    #[test]
    fn get_tag_value() {
        let mut tags = Map::new();
        tags.insert("foo".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("bar", element.tag("foo"));
        assert_eq!("", element.tag("missing"));
    }
}
