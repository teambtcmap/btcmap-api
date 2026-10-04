use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub user_name: String,
    pub geofence: Vec<i64>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub name: String,
    pub geofence: Vec<i64>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let target = db::main::user::queries::select_by_name(&params.user_name, pool).await?;
    let updated = db::main::user::queries::set_geofence(target.id, &params.geofence, pool).await?;
    Ok(Res {
        id: updated.id,
        name: updated.name,
        geofence: updated.geofence,
    })
}
