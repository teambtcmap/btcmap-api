use crate::{
    admin,
    area::Area,
    area_element::{self},
    discord,
    element::Element,
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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
    pub elements_processed: i64,
    pub elements_affected: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<Res> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let res = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_areas_elements_mapping(args.from_element_id, args.to_element_id, conn)
        })
        .await??;
    let log_message = format!(
            "{} generated areas to elements mappings, potentially affecting element ids {}..{}. {} elements were processed and {} elements were affected",
            admin.name, args.from_element_id, args.to_element_id, res.elements_processed, res.elements_affected
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
    let mut elements_processed = 0;
    let mut elements_affected = 0;
    let areas = Area::select_all(conn)?;
    for element_id in from_element_id..=to_element_id {
        let Some(element) = Element::select_by_id(element_id, conn)? else {
            continue;
        };
        if area_element::service::generate_areas_mapping(&element, &areas, conn)?.has_changes {
            elements_affected += 1;
        }
        elements_processed += 1;
    }
    Ok(Res {
        elements_processed,
        elements_affected,
    })
}
