use std::sync::Arc;

use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};

const NAME: &str = "get_elements_snapshot";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

#[derive(Serialize)]
pub struct SnapshotElement {
    pub osm_id: String,
    pub lat: f64,
    pub lon: f64,
    pub icon: String,
}

impl From<Element> for SnapshotElement {
    fn from(value: Element) -> Self {
        SnapshotElement {
            osm_id: value.overpass_data.btcmap_id(),
            lat: value.overpass_data.coord().y,
            lon: value.overpass_data.coord().x,
            icon: value
                .tag("icon:android")
                .as_str()
                .unwrap_or_default()
                .into(),
        }
    }
}

pub async fn run(
    Params(args): Params<Args>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<SnapshotElement>> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    let elements = pool
        .get()
        .await?
        .interact(move |conn| Element::select_all_except_deleted(conn))
        .await??;
    Ok(elements.into_iter().map(|it| it.into()).collect())
}
