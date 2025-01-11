use crate::{
    admin,
    area_element::{self, service::Diff},
    discord,
    element::Element,
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "generate_areas_elements_mapping";

#[derive(Deserialize)]
pub struct Args {
    password: String,
    from_element_id: i64,
    to_element_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub affected_elements: Vec<Diff>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let res = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_areas_elements_mapping(args.from_element_id, args.to_element_id, conn)
        })
        .await??;
    let log_message = format!(
            "{} generated areas to elements mappings, potentially affecting element ids {}..{}. {} elements were affected",
            admin.name, args.from_element_id, args.to_element_id, res.affected_elements.len(),
        );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(res)
}

fn generate_areas_elements_mapping(
    from_element_id: i64,
    to_element_id: i64,
    conn: &mut Connection,
) -> Result<Res> {
    let mut elements: Vec<Element> = vec![];
    for element_id in from_element_id..=to_element_id {
        let Some(element) = Element::select_by_id(element_id, conn)? else {
            continue;
        };
        elements.push(element);
    }
    let affected_elements = area_element::service::generate_mapping(&elements, conn)?;
    Ok(Res { affected_elements })
}
