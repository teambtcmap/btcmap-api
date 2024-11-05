use super::model::RpcArea;
use crate::{admin, area::Area, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;

const NAME: &str = "get_area";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<RpcArea> {
    admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let cloned_id = args.id.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&cloned_id, conn))
        .await??;
    area.map(|it| it.into())
        .ok_or(format!("There is no area with id or alias = {}", args.id).into())
}
