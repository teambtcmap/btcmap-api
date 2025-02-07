use crate::element::{self, model::Element};
use crate::Result;
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
}

#[derive(Serialize)]
pub struct Res {
    element: ResElement,
}

#[derive(Serialize)]
pub struct ResElement {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub tags: Map<String, Value>,
}

impl From<Element> for ResElement {
    fn from(val: Element) -> Self {
        let mut osm_tags = val.overpass_data.tags.clone().unwrap_or_default();
        osm_tags.sort_keys();
        let mut btcmap_tags = val.tags.clone();
        btcmap_tags.sort_keys();
        Self {
            id: val.id,
            lat: val.lat(),
            lon: val.lon(),
            tags: element::service::generate_tags(&val, &element::service::TAGS),
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    Ok(Res {
        element: Element::select_by_id_async(params.id, pool).await?.into(),
    })
}
