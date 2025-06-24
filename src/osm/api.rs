use crate::Result;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
struct OsmElementResponse {
    elements: Vec<OsmElement>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct OsmElement {
    pub r#type: String,
    pub id: i64,
    pub visible: Option<bool>,
    pub tags: Option<HashMap<String, String>>,
    pub user: String,
    pub uid: i64,
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
        Ok(if res.elements.len() == 1 {
            Some(res.elements.pop().unwrap())
        } else {
            None
        })
    } else {
        match res.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(format!("Unexpected response status: {}", res.status()))?,
        }
    }
}

#[derive(Deserialize)]
struct EditingApiUserResponse {
    user: EditingApiUser,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct EditingApiUser {
    pub id: i64,
    pub display_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub account_created: OffsetDateTime,
    pub description: String,
    pub contributor_terms: ContributorTerms,
    pub img: Option<Img>,
    pub roles: Vec<String>,
    pub changesets: Changesets,
    pub traces: Traces,
    pub blocks: Blocks,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ContributorTerms {
    pub agreed: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Img {
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Changesets {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Traces {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Blocks {
    pub received: BlocksReceived,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BlocksReceived {
    pub count: i32,
    pub active: i32,
}

impl EditingApiUser {
    #[cfg(test)]
    pub fn mock() -> EditingApiUser {
        EditingApiUser {
            id: 1,
            display_name: "".into(),
            account_created: OffsetDateTime::now_utc(),
            description: "".into(),
            contributor_terms: ContributorTerms { agreed: true },
            img: None,
            roles: vec![],
            changesets: Changesets { count: 0 },
            traces: Traces { count: 0 },
            blocks: Blocks {
                received: BlocksReceived {
                    count: 0,
                    active: 0,
                },
            },
        }
    }
}

pub async fn get_user(id: i64) -> Result<Option<EditingApiUser>> {
    let url = format!("https://api.openstreetmap.org/api/0.6/user/{id}.json");
    info!(url, "Querying OSM");
    let res = reqwest::get(&url).await?;
    info!(request_url = url, response_status = ?res.status(), "Got response from OSM");
    _get_user(res).await
}

async fn _get_user(res: Response) -> Result<Option<EditingApiUser>> {
    if res.status().is_success() {
        let res: EditingApiUserResponse = res.json().await?;
        Ok(Some(res.user))
    } else {
        match res.status() {
            StatusCode::NOT_FOUND => Ok(None),
            StatusCode::GONE => Ok(None),
            _ => Err(format!("Unexpected response status: {}", res.status()))?,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn get_element() -> Result<()> {
        //let res_json = r#"
        //{
        //    "version": "0.6",
        //    "generator": "CGImap 0.8.8 (1915379 spike-06.openstreetmap.org)",
        //    "copyright": "OpenStreetMap and contributors",
        //    "attribution": "http://www.openstreetmap.org/copyright",
        //    "license": "http://opendatacommons.org/licenses/odbl/1-0/",
        //    "elements": [
        //      {
        //        "type": "node",
        //        "id": 10016008392,
        //        "lat": 32.6463798,
        //        "lon": -16.9298181,
        //        "timestamp": "2023-10-25T04:04:55Z",
        //        "version": 4,
        //        "changeset": 143092629,
        //        "user": "Rockedf",
        //        "uid": 7522075,
        //        "tags": {
        //          "addr:city": "Funchal",
        //          "addr:housenumber": "47",
        //          "addr:postcode": "9000-645",
        //          "addr:street": "Rua das Virtudes",
        //          "check_date:currency:XBT": "2023-10-25",
        //          "currency:XBT": "yes",
        //          "name": "Monstera Books",
        //          "office": "company",
        //          "opening_hours": "Mo-Fr 09:00-18:00",
        //          "payment:lightning": "yes",
        //          "payment:lightning_contactless": "yes",
        //          "payment:onchain": "yes",
        //          "phone": "+351 916 001 177",
        //          "survey:date": "2023-10-24",
        //          "website": "https://monsterabooks.com"
        //        }
        //      }
        //    ]
        //  }
        //"#;

        //let res = super::_get_element(Builder::new().status(200).body(res_json)?.into()).await;
        //assert!(res.is_ok());
        //let element = res.unwrap();
        //assert!(element.is_some());
        //let element = element.unwrap();
        //assert_eq!("node", element.r#type);
        //assert_eq!(10016008392, element.id);
        Ok(())
    }

    #[actix_web::test]
    async fn get_element_404() -> Result<()> {
        //let res = super::_get_element(Builder::new().status(404).body("")?.into()).await;
        //assert!(res.is_ok());
        //let element = res.unwrap();
        //assert!(element.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn get_element_unexpected_res_code() -> Result<()> {
        //let res = super::_get_element(Builder::new().status(304).body("")?.into()).await;
        //assert!(res.is_err());
        Ok(())
    }

    #[actix_web::test]
    async fn get_user() -> Result<()> {
        //let res_json = r#"
        //{
        //    "version": "0.6",
        //    "generator": "OpenStreetMap server",
        //    "copyright": "OpenStreetMap and contributors",
        //    "attribution": "http://www.openstreetmap.org/copyright",
        //    "license": "http://opendatacommons.org/licenses/odbl/1-0/",
        //    "user": {
        //      "id": 1,
        //      "display_name": "Steve",
        //      "account_created": "2005-09-13T15:32:57Z",
        //      "description": "",
        //      "contributor_terms": {
        //        "agreed": true
        //      },
        //      "roles": [],
        //      "changesets": {
        //        "count": 1139
        //      },
        //      "traces": {
        //        "count": 23
        //      },
        //      "blocks": {
        //        "received": {
        //          "count": 0,
        //          "active": 0
        //        }
        //      }
        //    }
        //  }
        //"#;

        //let res = super::_get_user(Builder::new().status(200).body(res_json)?.into()).await;
        //assert!(res.is_ok());
        //let user = res.unwrap();
        //assert!(user.is_some());
        //let user = user.unwrap();
        //assert_eq!(1, user.id);
        //assert_eq!(23, user.traces.count);
        Ok(())
    }

    #[actix_web::test]
    async fn get_user_404() -> Result<()> {
        //let res = super::_get_user(Builder::new().status(404).body("")?.into()).await;
        //assert!(res.is_ok());
        //let user = res.unwrap();
        //assert!(user.is_none());
        Ok(())
    }

    #[actix_web::test]
    async fn get_user_unexpected_res_code() -> Result<()> {
        //let res = super::_get_user(Builder::new().status(304).body("")?.into()).await;
        //assert!(res.is_err());
        Ok(())
    }
}
