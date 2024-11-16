use super::model::RpcArea;
use crate::{admin, area::Area, Result};
use base64::prelude::*;
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::Map;
use std::{fs::OpenOptions, io::Write, sync::Arc};

const NAME: &str = "set_area_icon";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub icon_base64: String,
    pub icon_ext: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    let cloned_args_id = args.id.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&cloned_args_id, conn))
        .await??
        .ok_or(format!("There is no area with id or alias = {}", args.id))?;
    let file_name = format!("{}.{}", area.id, args.icon_ext);
    let bytes = BASE64_STANDARD.decode(args.icon_base64)?;
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
    let patch_set = Map::from_iter([("icon_square".into(), url.into())].into_iter());
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::patch_tags(area.id, patch_set, conn))
        .await??
        .ok_or("Failed to update area tag")?;
    Ok(area.into())
}
