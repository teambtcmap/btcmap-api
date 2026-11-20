use crate::Result;
use geo::{coord, Coord};
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::Duration;
use std::{hash::Hash, hash::Hasher};
use time::{
    format_description::well_known::Iso8601, macros::format_description, Date, OffsetDateTime,
};
use tracing::info;

static API_URL: &str = "https://overpass.btcmap.org/api/interpreter";
static USER_AGENT_VALUE: &str = "btcmap-api/1.0 (https://btcmap.org)";

static QUERY: &str = r#"
[out:json][timeout:600];
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

    #[cfg(test)]
    pub fn mock_with_tags(id: i64, tags: &[(&str, &str)]) -> OverpassElement {
        let mut map = Map::new();
        for (name, value) in tags {
            map.insert((*name).into(), (*value).into());
        }
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
            tags: Some(map),
            bounds: None,
            nodes: None,
            geometry: None,
            members: None,
        }
    }

    #[cfg(test)]
    pub fn mock_with_tag(id: i64, tag_name: &str, tag_value: &str) -> OverpassElement {
        let mut tags = Map::new();
        tags.insert(tag_name.into(), tag_value.into());
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
            tags: Some(tags),
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
    let started_at = OffsetDateTime::now_utc();
    // The Overpass query itself allows 600s (`[timeout:600]`), but the HTTP
    // client must still have a ceiling so a wedged connection can't hang the
    // sync indefinitely.
    let response = crate::service::http::client_with_timeout(Duration::from_secs(300))?
        .post(API_URL)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .body(QUERY)
        .send()
        .await?;
    info!(http_status_code = ?response.status(), "got overpass response");
    if response.status().as_u16() == 504 {
        Err("overpass server is busy")?
    }
    let response = response.json::<Response>().await?;
    if response.elements.is_empty() {
        Err(format!(
            "Got suspicious response: {}",
            serde_json::to_string_pretty(&response)?
        ))?
    }
    info!(elements = response.elements.len(), "Fetched elements");
    if response.elements.len() < 20000 {
        Err("Data set is most likely invalid")?
    }
    Ok(QueryBitcoinMerchantsRes {
        elements: response.elements,
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}

/// Fetch a single OSM element by type and id, including its centre coordinate.
///
/// Uses Overpass so that ways and relations resolve to a centre without extra
/// requests. Returns `None` when the element doesn't exist.
pub async fn get_element(osm_type: &str, osm_id: i64) -> Result<Option<OverpassElement>> {
    // `bb` (not `center`): `coord()` derives the centre from `bounds`, and the
    // response has no field to hold Overpass' `center` object.
    let query = format!("[out:json][timeout:10];\n{osm_type}({osm_id});\nout ids bb;");
    // The `[timeout:10]` above bounds query execution, not queueing at the
    // public instance, which alone can take tens of seconds.
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?
        .post(API_URL)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .body(query)
        .send()
        .await?;
    info!(http_status_code = ?response.status(), "got overpass element response");
    if !response.status().is_success() {
        Err(format!("unexpected overpass status: {}", response.status()))?;
    }
    let response = response.json::<Response>().await?;
    Ok(response.elements.into_iter().next())
}

#[cfg(test)]
mod test {
    use super::Bounds;
    use super::OverpassElement;
    use geo::coord;
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

    #[test]
    fn center_coord_uses_node_lat_lon() {
        let element = OverpassElement::mock(1);
        assert_eq!(coord! { x: 0.0, y: 0.0 }, element.coord());
    }

    #[test]
    fn center_coord_uses_bounds_for_ways() {
        let element = OverpassElement {
            r#type: "way".into(),
            lat: None,
            lon: None,
            bounds: Some(Bounds {
                minlon: 0.0,
                maxlon: 2.0,
                minlat: 4.0,
                maxlat: 6.0,
            }),
            ..OverpassElement::mock(1)
        };
        assert_eq!(coord! { x: 1.0, y: 5.0 }, element.coord());
    }

    #[test]
    fn center_coord_uses_bounds_for_relations() {
        let element = OverpassElement {
            r#type: "relation".into(),
            lat: None,
            lon: None,
            bounds: Some(Bounds {
                minlon: 0.0,
                maxlon: 2.0,
                minlat: 4.0,
                maxlat: 6.0,
            }),
            ..OverpassElement::mock(1)
        };
        assert_eq!(coord! { x: 1.0, y: 5.0 }, element.coord());
    }

    #[test]
    fn get_element_response_shape_has_bounds() {
        // `get_element` asks Overpass for `out ids bb`, which returns `bounds`
        // for ways and relations. Parsing the real shape keeps the query and
        // `coord()` in sync.
        let element: OverpassElement = serde_json::from_str(
            r#"{
                "type": "way",
                "id": 346185092,
                "bounds": {
                    "minlat": 13.0076801,
                    "minlon": 77.5981801,
                    "maxlat": 13.0077909,
                    "maxlon": 77.5982987
                }
            }"#,
        )
        .unwrap();

        let coord = element.coord();

        assert!((coord.x - 77.5982394).abs() < 1e-9);
        assert!((coord.y - 13.0077355).abs() < 1e-9);
    }
}
