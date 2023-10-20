use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::Value;
use tracing::warn;

use crate::model::Area;

use crate::Result;

pub fn run(conn: &Connection) -> Result<()> {
    for area in Area::select_all(None, &conn)? {
        let geo_json = area.tags.get("geo_json");

        if let Some(geo_json) = geo_json {
            if geo_json.is_string() {
                warn!(area.id, "Found improperly formatted geo_json tag");
                let unescaped = geo_json.as_str().unwrap().replace("\\\"", "\"");
                let geo_json: HashMap<String, Value> = serde_json::from_str(&unescaped)?;
                Area::insert_tag_as_json_obj(area.id, "geo_json", &geo_json, &conn)?;
                warn!(area.id, "Fixed geo_json tag");
            }
        }
    }

    Ok(())
}
