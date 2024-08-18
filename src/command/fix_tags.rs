use crate::{area::Area, Result};
use rusqlite::Connection;
use serde_json::{Map, Value};
use tracing::warn;

pub async fn run(conn: &Connection) -> Result<()> {
    for area in Area::select_all(None, conn)? {
        if let Some(geo_json) = area.tags.get("geo_json") {
            if geo_json.is_string() {
                warn!(area.id, "Found improperly formatted geo_json tag");
                let unescaped = geo_json.as_str().unwrap().replace("\\\"", "\"");
                let geo_json: Value = serde_json::from_str(&unescaped)?;
                let mut patch_set = Map::new();
                patch_set.insert("geo_json".into(), geo_json);
                Area::patch_tags(area.id, patch_set, &conn)?;
                warn!(area.id, "Fixed geo_json tag");
            }
        }
    }
    Ok(())
}
