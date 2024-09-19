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
use tracing::warn;

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
    let tx: Transaction = conn.transaction()?;
    let cached_elements = Element::select_all(None, &tx)?;
    for fresh_element in fresh_overpass_elements {
        let btcmap_id = fresh_element.btcmap_id();
        let user_id = fresh_element.uid;

        match cached_elements
            .iter()
            .find(|it| it.overpass_data.btcmap_id() == btcmap_id)
        {
            Some(cached_element) => {
                if fresh_element != cached_element.overpass_data {
                    info!(
                        btcmap_id,
                        old_json = serde_json::to_string(&cached_element.overpass_data)?,
                        new_json = serde_json::to_string(&fresh_element)?,
                        "Element JSON was updated",
                    );

                    if let Some(user_id) = user_id {
                        sync::insert_user_if_not_exists(user_id, &tx).await?;
                    }

                    if fresh_element.changeset != cached_element.overpass_data.changeset {
                        let event = Event::insert(
                            user_id.unwrap().try_into().unwrap(),
                            cached_element.id,
                            "update",
                            &tx,
                        )?;
                        event::service::on_new_event(&event, &tx).await?;
                    } else {
                        warn!("Changeset ID is identical, skipped user event generation");
                    }

                    info!("Updating osm_json");
                    let mut updated_element =
                        cached_element.set_overpass_data(&fresh_element, &tx)?;

                    let new_android_icon = updated_element.overpass_data.generate_android_icon();
                    let old_android_icon = cached_element
                        .tag("icon:android")
                        .as_str()
                        .unwrap_or_default();

                    if new_android_icon != old_android_icon {
                        info!(old_android_icon, new_android_icon, "Updating Android icon");
                        updated_element = Element::set_tag(
                            updated_element.id,
                            "icon:android",
                            &new_android_icon.clone().into(),
                            &tx,
                        )?;
                    }

                    element::service::generate_issues(vec![&updated_element], &tx)?;
                    element::service::generate_areas_mapping_old(&vec![updated_element], &tx)?;
                }

                if cached_element.deleted_at.is_some() {
                    info!(btcmap_id, "Bitcoin tags were re-added");
                    cached_element.set_deleted_at(None, &tx)?;
                }
            }
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
