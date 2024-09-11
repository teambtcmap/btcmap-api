use crate::{
    area::Area,
    area_element::model::AreaElement,
    auth::Token,
    discord,
    element::{self, Element},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    token: String,
    from_element_id: i64,
    to_element_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub elements_processed: i64,
    pub elements_affected: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let res = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_areas_elements_mapping(args.from_element_id, args.to_element_id, conn)
        })
        .await??;
    let log_message = format!(
            "{} generated areas to elements mappings, potentially affecting element ids {}..{}. {} elements were processed and {} elements were affected",
            token.owner, args.from_element_id, args.to_element_id, res.elements_processed, res.elements_affected
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
    let areas = Area::select_all(None, conn)?;
    for element_id in from_element_id..=to_element_id {
        let element = Element::select_by_id(element_id, conn)?;
        if element.is_none() {
            break;
        }
        let element = element.unwrap();
        let element_areas = element::service::find_areas(&element, &areas)?;
        let old_mappings = AreaElement::select_by_element_id(element_id, conn)?;
        let mut old_area_ids: Vec<i64> = old_mappings.into_iter().map(|it| it.area_id).collect();
        let mut new_area_ids: Vec<i64> = element_areas.into_iter().map(|it| it.id).collect();
        old_area_ids.sort();
        new_area_ids.sort();
        let sp = conn.savepoint()?;
        if new_area_ids != old_area_ids {
            for old_area_id in &old_area_ids {
                if !new_area_ids.contains(&old_area_id) {
                    AreaElement::set_deleted_at(
                        *old_area_id,
                        Some(OffsetDateTime::now_utc()),
                        &sp,
                    )?;
                }
            }
            for new_area_id in new_area_ids {
                if !old_area_ids.contains(&new_area_id) {
                    AreaElement::insert(new_area_id, element_id, &sp)?;
                }
            }
            elements_affected += 1;
        }
        sp.commit()?;
        elements_processed += 1;
    }
    Ok(Res {
        elements_processed,
        elements_affected,
    })
}
