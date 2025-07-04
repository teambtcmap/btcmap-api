use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde_json::{Map, Value};
use tracing::warn;

pub async fn enforce_v2_compat(pool: &Pool) -> Result<()> {
    for report in db::report::queries_async::select_all(None, None, pool).await? {
        if report.tags.get("area_url_alias").is_none() {
            warn!(id = report.id, "Report is not v2 compatible, upgrading");
            let area = db::area::queries_async::select_by_id(report.area_id, pool).await?;
            let mut tags_to_merge: Map<String, Value> = Map::new();
            tags_to_merge.insert("area_url_alias".into(), area.alias().into());
            db::report::queries_async::patch_tags(report.id, tags_to_merge, pool).await?;
        }
    }
    Ok(())
}
