use std::{collections::HashMap, ops::Sub};

use geo::{coord, Coord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Iso8601, Duration, OffsetDateTime};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct OverpassElement {
    pub r#type: String,
    pub id: i64,
    pub lat: Option<f64>, // for nodes only
    pub lon: Option<f64>, // for nodes only
    pub timestamp: Option<String>,
    pub version: Option<i64>,
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub uid: Option<i64>,
    pub tags: Option<HashMap<String, String>>,
    pub bounds: Option<Bounds>,  // for ways and relations only
    pub nodes: Option<Value>,    // for ways only
    pub geometry: Option<Value>, // for ways only
    pub members: Option<Value>,  // for relations only
}

#[derive(Serialize, Deserialize, PartialEq)]
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
        let verification_date = self
            .verification_date()
            .map(|it| it.to_string().to_string())
            .unwrap_or(String::new());
        let year_ago = OffsetDateTime::now_utc().date().sub(Duration::days(365));
        verification_date.as_str() > year_ago.to_string().as_str()
    }

    pub fn verification_date(&self) -> Option<OffsetDateTime> {
        let survey_date = self.get_tag_value("survey:date");
        let check_date = self.get_tag_value("check_date");
        let bitcoin_check_date = self.get_tag_value("check_date:currency:XBT");

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

    pub fn coord(&self) -> Coord {
        match self.r#type.as_str() {
            "node" => coord! { x: self.lon.unwrap(), y: self.lat.unwrap() },
            _ => {
                let bounds = self.bounds.as_ref().unwrap();
                coord! { x: (bounds.minlon + bounds.maxlon) / 2.0, y: (bounds.minlat + bounds.maxlat) / 2.0 }
            }
        }
    }

    pub fn get_tag_value(&self, name: &str) -> &str {
        match &self.tags {
            Some(tags) => tags.get(name).map(|it| it.as_str()).unwrap_or(""),
            None => "",
        }
    }

    #[cfg(test)]
    pub fn mock() -> OverpassElement {
        OverpassElement {
            r#type: "node".into(),
            id: 1,
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
            ..OverpassElement::mock()
        };
        assert_eq!("bar", element.get_tag_value("foo"));
        assert_eq!("", element.get_tag_value("missing"));
    }
}
