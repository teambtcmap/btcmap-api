use crate::element::model::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

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
    pub osm_url: String,
    pub osm_tags: Map<String, Value>,
    pub btcmap_tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
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
            osm_url: val.osm_url(),
            osm_tags,
            btcmap_tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    Ok(Res {
        element: Element::select_by_id_async(params.id, pool).await?.into(),
    })
}
