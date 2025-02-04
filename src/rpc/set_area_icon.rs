use super::model::RpcArea;
use crate::{admin, area::Area, Result};
use base64::prelude::*;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::Map;
use std::{fs::OpenOptions, io::Write};

pub const NAME: &str = "set_area_icon";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
    pub icon_base64: String,
    pub icon_ext: String,
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<RpcArea> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let area = Area::select_by_id_or_alias_async(&params.id, &pool).await?;
    let file_name = format!("{}.{}", area.id, params.icon_ext);
    let bytes = BASE64_STANDARD.decode(params.icon_base64)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!(
            "/srv/http/static.btcmap.org/images/areas/{file_name}"
        ))?;
    file.write_all(&bytes)?;
    file.flush()?;
    let url = format!("https://static.btcmap.org/images/areas/{file_name}");
    let patch_set = Map::from_iter([("icon:square".into(), url.into())].into_iter());
    let area = Area::patch_tags_async(area.id, patch_set, &pool).await?;
    Ok(area.into())
}
