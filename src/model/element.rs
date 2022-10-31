use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Element {
    pub id: String,
    pub osm_json: Value,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

// impl Element {
//     pub fn lat(&self) -> f64 {
//         match self.data["type"].as_str().unwrap() {
//             "node" => self.data["lat"].as_f64().unwrap(),
//             _ => {
//                 let min_lat = self.data["bounds"]["minlat"].as_f64().unwrap();
//                 let max_lat = self.data["bounds"]["maxlat"].as_f64().unwrap();
//                 (min_lat + max_lat) / 2.0
//             }
//         }
//     }

//     pub fn lon(&self) -> f64 {
//         match self.data["type"].as_str().unwrap() {
//             "node" => self.data["lon"].as_f64().unwrap(),
//             _ => {
//                 let min_lon = self.data["bounds"]["minlon"].as_f64().unwrap();
//                 let max_lon = self.data["bounds"]["maxlon"].as_f64().unwrap();
//                 (min_lon + max_lon) / 2.0
//             }
//         }
//     }
// }
