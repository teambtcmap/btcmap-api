use crate::osm::overpass::OverpassElement;
use serde_json::{Map, Value};
use std::hash::Hash;
use std::hash::Hasher;
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct Element {
    pub id: i64,
    pub overpass_data: OverpassElement,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Element {}

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Element {
    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }

    pub fn name(&self) -> String {
        self.overpass_data.tag("name").into()
    }

    pub fn osm_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/{}/{}",
            self.overpass_data.r#type, self.overpass_data.id,
        )
    }

    pub fn lat(&self) -> f64 {
        self.overpass_data.coord().y
    }

    pub fn lon(&self) -> f64 {
        self.overpass_data.coord().x
    }
}
