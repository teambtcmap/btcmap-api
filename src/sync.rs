use crate::element::{self, Element};
use crate::event::{self, Event};
use crate::osm::overpass::OverpassElement;
use crate::osm::{self, api::OsmElement};
use crate::user::User;
use crate::{area_element, discord, Error, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use tracing::{error, info, warn};

#[derive(Serialize)]
pub struct MergeResult {
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
}

pub async fn merge_overpass_elements(
    fresh_overpass_elements: Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<MergeResult> {
    // stage 1: find and process deleted elements
    let fresh_elemement_ids: HashSet<String> = fresh_overpass_elements
        .iter()
        .map(|it| it.btcmap_id())
        .collect();
    let deleted_element_events = sync_deleted_elements(&fresh_elemement_ids, conn).await?;
    // stage 2: find and process updated elements
    let updated_element_events = sync_updated_elements(&fresh_overpass_elements, conn).await?;
    // stage 3: find and process new elements
    let created_element_evnets = sync_new_elements(&fresh_overpass_elements, conn).await?;
    Ok(MergeResult {
        elements_created: created_element_evnets.len() as i64,
        elements_updated: updated_element_events.len() as i64,
        elements_deleted: deleted_element_events.len() as i64,
    })
}

/// Match fresh Overpass elements with the existing cached elements. Every
/// cached element which isn't present in Overpass list will be marked as
/// deleted.
///
/// # Failure
///
/// Will return `Err` in many cases!
pub async fn sync_deleted_elements(
    fresh_overpass_element_ids: &HashSet<String>,
    conn: &mut Connection,
) -> Result<Vec<Event>> {
    let absent_elements: Vec<Element> = Element::select_all_except_deleted(conn)?
        .into_iter()
        .filter(|it| !fresh_overpass_element_ids.contains(&it.overpass_data.btcmap_id()))
        .collect();
    let mut res = vec![];
    for absent_element in absent_elements {
        let fresh_osm_element = confirm_deleted(
            &absent_element.overpass_data.r#type,
            absent_element.overpass_data.id,
        )
        .await?;
        insert_user_if_not_exists(fresh_osm_element.uid, conn).await?;
        res.push(mark_element_as_deleted(
            &absent_element,
            &fresh_osm_element,
            conn,
        )?);
    }
    for event in &res {
        event::service::on_new_event(event, conn).await?;
    }
    Ok(res)
}

fn mark_element_as_deleted(
    element: &Element,
    fresh_osm_element: &OsmElement,
    conn: &mut Connection,
) -> Result<Event> {
    let mut event_tags: HashMap<String, Value> = HashMap::new();
    event_tags.insert(
        "element_osm_type".into(),
        element.overpass_data.r#type.clone().into(),
    );
    event_tags.insert("element_osm_id".into(), element.overpass_data.id.into());
    event_tags.insert("element_name".into(), element.name().into());
    if element.tags.contains_key("areas") {
        event_tags.insert("areas".into(), element.tags["areas"].clone());
    }
    let sp = conn.savepoint()?;
    element.set_deleted_at(Some(OffsetDateTime::now_utc()), &sp)?;
    let event = Event::insert(fresh_osm_element.uid, element.id, "delete", &sp)?;
    let event = event.patch_tags(&event_tags, &sp)?;
    sp.commit()?;
    Ok(event)
}

async fn confirm_deleted(osm_type: &str, osm_id: i64) -> Result<OsmElement> {
    let osm_element = match osm::api::get_element(osm_type, osm_id).await? {
        Some(v) => v,
        None => Err(Error::OsmApi(format!(
            "Failed to fetch element {}:{} from OSM",
            osm_type, osm_id,
        )))?,
    };
    if osm_element.visible.unwrap_or(true) && osm_element.tag("currency:XBT", "no") == "yes" {
        let message = format!(
            "Overpass lied about element {}:{} being deleted",
            osm_type, osm_id,
        );
        error!(message);
        discord::send_message_to_channel(&message, discord::CHANNEL_OSM_CHANGES).await;
        Err(Error::OverpassApi(message))?
    }
    Ok(osm_element)
}

pub async fn sync_updated_elements(
    fresh_overpass_elements: &Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<Vec<Event>> {
    let mut res = vec![];
    let cached_elements = Element::select_all(None, conn)?;
    for fresh_element in fresh_overpass_elements {
        let btcmap_id = fresh_element.btcmap_id();
        let user_id = fresh_element.uid;
        let cached_element = cached_elements
            .iter()
            .find(|it| it.overpass_data.btcmap_id() == fresh_element.btcmap_id());
        if cached_element.is_none() {
            continue;
        }
        let mut cached_element = cached_element.unwrap().clone();
        if cached_element.deleted_at.is_some() {
            info!(btcmap_id, "Bitcoin tags were re-added");
            cached_element = cached_element.set_deleted_at(None, conn)?;
        }
        if *fresh_element == cached_element.overpass_data {
            continue;
        }
        info!(
            btcmap_id,
            old_json = serde_json::to_string(&cached_element.overpass_data)?,
            new_json = serde_json::to_string(&fresh_element)?,
            "Element JSON was updated",
        );
        if let Some(user_id) = user_id {
            insert_user_if_not_exists(user_id, conn).await?;
        }
        let sp = conn.savepoint()?;
        if fresh_element.changeset != cached_element.overpass_data.changeset {
            let event = Event::insert(user_id.unwrap(), cached_element.id, "update", &sp)?;
            res.push(event);
        } else {
            warn!("Changeset ID is identical, skipped user event generation");
        }
        info!("Updating osm_json");
        let mut updated_element = cached_element.set_overpass_data(fresh_element, &sp)?;
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
                &sp,
            )?;
        }
        element::service::generate_issues(vec![&updated_element], &sp)?;
        area_element::service::generate_mapping(&vec![updated_element], &sp)?;
        sp.commit()?;
    }
    for event in &res {
        event::service::on_new_event(event, conn).await?;
    }
    Ok(res)
}

pub async fn sync_new_elements(
    fresh_overpass_elements: &Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<Vec<Event>> {
    let mut res = vec![];
    let cached_elements = Element::select_all(None, conn)?;
    for fresh_element in fresh_overpass_elements {
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
                    insert_user_if_not_exists(user_id, conn).await?;
                }
                let sp = conn.savepoint()?;
                let element = Element::insert(fresh_element, &sp)?;
                let event = Event::insert(user_id.unwrap(), element.id, "create", &sp)?;
                res.push(event);
                let category = element.overpass_data.generate_category();
                let android_icon = element.overpass_data.generate_android_icon();
                let element =
                    Element::set_tag(element.id, "category", &category.clone().into(), &sp)?;
                let element = Element::set_tag(
                    element.id,
                    "icon:android",
                    &android_icon.clone().into(),
                    &sp,
                )?;
                info!(category, android_icon);
                element::service::generate_issues(vec![&element], &sp)?;
                area_element::service::generate_mapping(&vec![element], &sp)?;
                sp.commit()?;
            }
        }
    }
    for event in &res {
        event::service::on_new_event(event, conn).await?;
    }
    Ok(res)
}

async fn insert_user_if_not_exists(user_id: i64, conn: &Connection) -> Result<()> {
    if User::select_by_id(user_id, conn)?.is_some() {
        info!(user_id, "User already exists");
        return Ok(());
    }
    match osm::api::get_user(user_id).await? {
        Some(user) => User::insert(user_id, &user, conn)?,
        None => Err(Error::OsmApi(format!(
            "User with id = {user_id} doesn't exist on OSM"
        )))?,
    };
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        element::Element,
        osm::{api::OsmUser, overpass::OverpassElement},
        test::mock_conn,
        user::User,
        Result,
    };
    use actix_web::test;

    #[test]
    #[ignore = "relies on external service"]
    async fn sync_deleted_elements() -> Result<()> {
        let mut conn = mock_conn();
        let _element_1 = Element::insert(&OverpassElement::mock(1), &conn)?;
        let _element_2 = Element::insert(&OverpassElement::mock(2), &conn)?;
        let element_3 = Element::insert(&OverpassElement::mock(2702291726), &conn)?;
        let res = super::sync_deleted_elements(
            &vec!["node:1".to_string(), "node:2".to_string()]
                .into_iter()
                .collect(),
            &mut conn,
        )
        .await;
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.len() == 1);
        let res = res.first().unwrap();
        assert_eq!(
            element_3.overpass_data.btcmap_id(),
            format!("{}:{}", res.element_osm_type, res.element_osm_id),
        );
        Ok(())
    }

    #[test]
    #[ignore = "relies on external service"]
    async fn confirm_deleted() -> Result<()> {
        assert!(super::confirm_deleted("node", 2702291726).await.is_ok());
        assert!(super::confirm_deleted("node", 12181429828).await.is_err());
        Ok(())
    }

    #[test]
    async fn insert_user_if_not_exists_when_cached() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        assert!(super::insert_user_if_not_exists(user.id, &conn)
            .await
            .is_ok());
        Ok(())
    }

    #[test]
    #[ignore = "relies on external service"]
    async fn insert_user_if_not_exists_when_exists_on_osm() -> Result<()> {
        let conn = mock_conn();
        let btc_map_user_id = 18545877;
        assert!(super::insert_user_if_not_exists(btc_map_user_id, &conn)
            .await
            .is_ok());
        assert!(User::select_by_id(btc_map_user_id, &conn)?.is_some());
        Ok(())
    }
}
