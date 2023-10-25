use std::collections::HashMap;

use reqwest::{Response, StatusCode};
use serde::Deserialize;
use tracing::info;

use crate::{Error, Result};

#[derive(Deserialize)]
struct OsmElementResponse {
    elements: Vec<OsmElement>,
}

#[derive(Deserialize)]
pub struct OsmElement {
    pub r#type: String,
    pub id: i64,
    pub visible: Option<bool>,
    pub tags: Option<HashMap<String, String>>,
    pub user: String,
    pub uid: i32,
}

impl OsmElement {
    pub fn tag(&self, name: &str, default: &str) -> String {
        match &self.tags {
            Some(tags) => tags.get(name).map(|it| it.into()).unwrap_or(default.into()),
            None => default.into(),
        }
    }
}

pub async fn get_element(element_type: &str, element_id: i64) -> Result<Option<OsmElement>> {
    let url = format!(
        "https://api.openstreetmap.org/api/0.6/{element_type}s.json?{element_type}s={element_id}"
    );
    info!(url, "Querying OSM");
    let res = reqwest::get(&url).await?;
    info!(request_url = url, response_status = ?res.status(), "Got response from OSM");
    _get_element(res).await
}

async fn _get_element(res: Response) -> Result<Option<OsmElement>> {
    if res.status().is_success() {
        let mut res: OsmElementResponse = res.json().await?;
        return Ok(if res.elements.len() == 1 {
            Some(res.elements.pop().unwrap())
        } else {
            None
        });
    } else {
        match res.status() {
            StatusCode::NOT_FOUND => return Ok(None),
            _ => Err(Error::Other(format!(
                "Unexpected response status: {}",
                res.status()
            )))?,
        }
    }
}

#[cfg(test)]
mod test {
    use http::response::Builder;

    use crate::Result;

    #[actix_web::test]
    async fn get_element() -> Result<()> {
        let res_json = r#"
        {
            "version": "0.6",
            "generator": "CGImap 0.8.8 (1915379 spike-06.openstreetmap.org)",
            "copyright": "OpenStreetMap and contributors",
            "attribution": "http://www.openstreetmap.org/copyright",
            "license": "http://opendatacommons.org/licenses/odbl/1-0/",
            "elements": [
              {
                "type": "node",
                "id": 10016008392,
                "lat": 32.6463798,
                "lon": -16.9298181,
                "timestamp": "2023-10-25T04:04:55Z",
                "version": 4,
                "changeset": 143092629,
                "user": "Rockedf",
                "uid": 7522075,
                "tags": {
                  "addr:city": "Funchal",
                  "addr:housenumber": "47",
                  "addr:postcode": "9000-645",
                  "addr:street": "Rua das Virtudes",
                  "check_date:currency:XBT": "2023-10-25",
                  "currency:XBT": "yes",
                  "name": "Monstera Books",
                  "office": "company",
                  "opening_hours": "Mo-Fr 09:00-18:00",
                  "payment:lightning": "yes",
                  "payment:lightning_contactless": "yes",
                  "payment:onchain": "yes",
                  "phone": "+351 916 001 177",
                  "survey:date": "2023-10-24",
                  "website": "https://monsterabooks.com"
                }
              }
            ]
          }
        "#;

        let res = super::_get_element(Builder::new().status(200).body(res_json)?.into()).await;
        assert!(res.is_ok());
        let element = res.unwrap();
        assert!(element.is_some());
        let element = element.unwrap();
        assert_eq!("node", element.r#type);
        assert_eq!(10016008392, element.id);
        Ok(())
    }

    #[actix_web::test]
    async fn get_element_404() -> Result<()> {
        let res = super::_get_element(Builder::new().status(404).body("")?.into()).await;
        assert!(res.is_ok());
        let element = res.unwrap();
        assert!(element.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn get_element_unexpected_res_code() -> Result<()> {
        let res = super::_get_element(Builder::new().status(304).body("")?.into()).await;
        assert!(res.is_err());
        Ok(())
    }
}
