use rusqlite::Connection;
use serde_json::{Map, Value};
use tracing::warn;

use crate::model::Area;

use crate::Result;

pub fn run(conn: &Connection) -> Result<()> {
    let areas = Area::select_all(&conn, None)?;

    for area in areas {
        let geo_json = area.tag("geo_json");

        if geo_json.is_string() {
            warn!(area.id, "Found improperly formatted geo_json tag");
            let unescaped = geo_json.as_str().unwrap().replace("\\\"", "\"");
            let geo_json: Map<String, Value> = serde_json::from_str(&unescaped)?;
            area.insert_tag_json("geo_json", &Value::Object(geo_json), &conn)?;
            warn!(area.id, "Fixed geo_json tag");
        }
    }

    Ok(())
}
