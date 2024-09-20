use crate::element;
use crate::element::Element;
use crate::event;
use crate::event::Event;
use crate::osm::overpass::query_bitcoin_merchants;
use crate::osm::overpass::OverpassElement;
use crate::sync;
use crate::Result;
use rusqlite::Connection;
use rusqlite::Transaction;
use tracing::info;

pub async fn run(conn: &mut Connection) -> Result<()> {
    let elements = query_bitcoin_merchants().await?;
    let res = merge_overpass_elements(elements, conn).await?;
    info!(res.elements_deleted);
    Ok(())
}

pub struct MergeResult {
    pub elements_deleted: usize,
}

async fn merge_overpass_elements(
    fresh_overpass_elements: Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<MergeResult> {
    // stage 1: find and process deleted elements
    let deleted_element_events = sync::sync_deleted_elements(
        &fresh_overpass_elements
            .iter()
            .map(|it| it.btcmap_id())
            .collect(),
        conn,
    )
    .await?;
    for event in &deleted_element_events {
        event::service::on_new_event(&event, conn).await?;
    }

    // stage 2: find and process updated elements
    sync::sync_updated_elements(&fresh_overpass_elements, conn).await?;

    // stage 3: find and process new elements
    let tx: Transaction = conn.transaction()?;
    let cached_elements = Element::select_all(None, &tx)?;
    for fresh_element in &fresh_overpass_elements {
        let btcmap_id = fresh_element.btcmap_id();
        let user_id = fresh_element.uid;

        match cached_elements
            .iter()
            .find(|it| it.overpass_data.btcmap_id() == btcmap_id)
        {
            Some(_) => {}
            None => {
                info!(btcmap_id, "Element does not exist, inserting");

                if let Some(user_id) = user_id {
                    sync::insert_user_if_not_exists(user_id, &tx).await?;
                }

                let element = Element::insert(&fresh_element, &tx)?;

                let event = Event::insert(
                    user_id.unwrap().try_into().unwrap(),
                    element.id,
                    "create",
                    &tx,
                )?;
                event::service::on_new_event(&event, &tx).await?;

                let category = element.overpass_data.generate_category();
                let android_icon = element.overpass_data.generate_android_icon();

                let element =
                    Element::set_tag(element.id, "category", &category.clone().into(), &tx)?;
                let element = Element::set_tag(
                    element.id,
                    "icon:android",
                    &android_icon.clone().into(),
                    &tx,
                )?;

                info!(category, android_icon);

                element::service::generate_issues(vec![&element], &tx)?;
                element::service::generate_areas_mapping_old(&vec![element], &tx)?;
            }
        }
    }
    tx.commit()?;

    Ok(MergeResult {
        elements_deleted: deleted_element_events.len(),
    })
}
