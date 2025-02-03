use crate::{admin, element::Element, event::Event, user::User, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

pub const NAME: &str = "get_user_activity";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
    pub limit: i64,
}

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    pub message: String,
    pub osm_url: String,
    pub btcmap_url: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<Res>> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let cloned_args_id = params.id.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::select_by_id_or_name(&cloned_args_id, conn))
        .await??
        .ok_or(format!("There is no user with id or name = {}", params.id))?;
    let events = pool
        .get()
        .await?
        .interact(move |conn| Event::select_by_user(user.id, params.limit, conn))
        .await??;
    let events_elements: Vec<(Event, Element)> = pool
        .get()
        .await?
        .interact(move |conn| {
            events
                .into_iter()
                .map(|it| {
                    let cloned_id = it.element_id;
                    (it, Element::select_by_id(cloned_id, conn).unwrap().unwrap())
                    // TODO remove unwraps
                })
                .collect()
        })
        .await?;
    let res = events_elements
        .into_iter()
        .map(|it| Res {
            date: it.0.created_at,
            message: format!(
                "{} {}d element {}",
                user.osm_data.display_name,
                it.0.r#type,
                it.1.name(),
            ),
            osm_url: format!(
                "https://www.openstreetmap.org/{}/{}",
                it.0.element_osm_type, it.0.element_osm_id
            ),
            btcmap_url: format!(
                "https://btcmap.org/merchant/{}:{}",
                it.0.element_osm_type, it.0.element_osm_id
            ),
        })
        .collect();
    Ok(res)
}
