use crate::{area::AreaRepo, Result};
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;

pub async fn run(repo: &AreaRepo) -> Result<()> {
    for area in repo.select_all(None).await? {
        if let Some(geo_json) = area.tags.get("geo_json") {
            if geo_json.is_string() {
                warn!(area.id, "Found improperly formatted geo_json tag");
                let unescaped = geo_json.as_str().unwrap().replace("\\\"", "\"");
                let geo_json: Value = serde_json::from_str(&unescaped)?;
                let mut patch_set = HashMap::new();
                patch_set.insert("geo_json".into(), geo_json);
                repo.patch_tags(area.id, &patch_set).await?;
                warn!(area.id, "Fixed geo_json tag");
            }
        }
    }
    Ok(())
}
