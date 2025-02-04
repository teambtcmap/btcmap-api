use crate::element::model::Element;
use crate::element::v4::GetItem;
use crate::{element, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<element::v4::GetItem> {
    Element::select_by_id_or_osm_id_async(&params.id, pool)
        .await?
        .map(|it| GetItem {
            id: it.id,
            lat: it.overpass_data.coord().y,
            lon: it.overpass_data.coord().x,
            tags: element::v4::generate_tags(&it, &vec!["name".into()]),
            updated_at: it.updated_at,
            deleted_at: it.deleted_at,
        })
        .ok_or(format!("There is no element with id {}", params.id).into())
}
