use crate::area_element::service::Diff;
use crate::conf::Conf;
use crate::element::{self, Element};
use crate::event::{self, Event};
use crate::osm::overpass::OverpassElement;
use crate::osm::{self, api::OsmElement};
use crate::{area_element, discord, user, Error, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use tracing::error;

#[derive(Serialize)]
pub struct MergeResult {
    pub elements_created: Vec<MergeResultElement>,
    pub elements_updated: Vec<MergeResultElement>,
    pub elements_deleted: Vec<MergeResultElement>,
    pub total_time_s: f64,
    pub deleted_sync_time_s: f64,
    pub created_sync_time_s: f64,
    pub updated_sync_time_s: f64,
    pub events_processing_time_s: f64,
    pub area_mapping_processing_time_s: f64,
    pub area_mapping_diff: Vec<Diff>,
}

#[derive(Serialize)]
pub struct MergeResultElement {
    pub id: i64,
    pub osm_url: String,
    pub name: String,
}

impl From<Element> for MergeResultElement {
    fn from(val: Element) -> Self {
        MergeResultElement {
            id: val.id,
            osm_url: val.osm_url(),
            name: val.name(),
        }
    }
}

pub async fn merge_overpass_elements(
    fresh_overpass_elements: Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<MergeResult> {
    let started_at = OffsetDateTime::now_utc();
    // stage 1: find and process deleted elements
    let deleted_element_events = sync_deleted_elements(&fresh_overpass_elements, conn).await?;
    let deleted_elements: Vec<Element> = deleted_element_events
        .iter()
        .map(|it| Element::select_by_id(it.element_id, conn).unwrap().unwrap())
        .collect();
    let deleted_elements: Vec<MergeResultElement> =
        deleted_elements.into_iter().map(|it| it.into()).collect();
    let deleted_sync_time_s = (OffsetDateTime::now_utc() - started_at).as_seconds_f64();

    // stage 2: find and process updated elements
    let updated_sync_started_at = OffsetDateTime::now_utc();
    let updated_element_events = sync_updated_elements(&fresh_overpass_elements, conn).await?;
    let updated_elements: Vec<Element> = updated_element_events
        .iter()
        .map(|it| Element::select_by_id(it.element_id, conn).unwrap().unwrap())
        .collect();
    let updated_sync_time_s =
        (OffsetDateTime::now_utc() - updated_sync_started_at).as_seconds_f64();

    // stage 3: find and process new elements
    let created_sync_started_at = OffsetDateTime::now_utc();
    let created_element_events = sync_new_elements(&fresh_overpass_elements, conn).await?;
    let created_elements: Vec<Element> = created_element_events
        .iter()
        .map(|it| Element::select_by_id(it.element_id, conn).unwrap().unwrap())
        .collect();
    let created_sync_time_s =
        (OffsetDateTime::now_utc() - created_sync_started_at).as_seconds_f64();

    let events_processing_started_at = OffsetDateTime::now_utc();
    let mut all_events: Vec<Event> = vec![];
    all_events.extend(created_element_events);
    all_events.extend(updated_element_events);
    all_events.extend(deleted_element_events);
    for event in all_events {
        event::service::on_new_event(&event, conn).await?;
    }
    let events_processing_time_s =
        (OffsetDateTime::now_utc() - events_processing_started_at).as_seconds_f64();

    let area_mapping_started_at = OffsetDateTime::now_utc();
    let mut area_mapping_elements: Vec<Element> = vec![];
    area_mapping_elements.extend(created_elements.clone());
    area_mapping_elements.extend(updated_elements.clone());
    let area_mapping_diff = area_element::service::generate_mapping(&area_mapping_elements, conn)?;
    let area_mapping_processing_time_s =
        (OffsetDateTime::now_utc() - area_mapping_started_at).as_seconds_f64();

    let created_elements: Vec<MergeResultElement> =
        created_elements.into_iter().map(|it| it.into()).collect();
    let updated_elements: Vec<MergeResultElement> =
        updated_elements.into_iter().map(|it| it.into()).collect();

    Ok(MergeResult {
        elements_created: created_elements,
        elements_updated: updated_elements,
        elements_deleted: deleted_elements,
        total_time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
        deleted_sync_time_s,
        updated_sync_time_s,
        created_sync_time_s,
        events_processing_time_s,
        area_mapping_processing_time_s,
        area_mapping_diff,
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
    fresh_overpass_elements: &Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<Vec<Event>> {
    let fresh_overpass_element_ids: HashSet<String> = fresh_overpass_elements
        .iter()
        .map(|it| it.btcmap_id())
        .collect();
    let absent_elements: Vec<Element> = Element::select_all_except_deleted(conn)?
        .into_iter()
        .filter(|it| !fresh_overpass_element_ids.contains(&it.overpass_data.btcmap_id()))
        .collect();
    let mut res = vec![];
    let conf = Conf::select(conn)?;
    for absent_element in absent_elements {
        let fresh_osm_element = confirm_deleted(
            &absent_element.overpass_data.r#type,
            absent_element.overpass_data.id,
            &conf,
        )
        .await?;
        user::service::insert_user_if_not_exists(fresh_osm_element.uid, conn).await?;
        res.push(mark_element_as_deleted(
            &absent_element,
            &fresh_osm_element,
            conn,
        )?);
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

async fn confirm_deleted(osm_type: &str, osm_id: i64, conf: &Conf) -> Result<OsmElement> {
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
        discord::post_message(&conf.discord_webhook_osm_changes, &message).await;
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
    for fresh_overpass_element in fresh_overpass_elements {
        let cached_element = cached_elements.iter().find(|cached_element| {
            cached_element.overpass_data.r#type == fresh_overpass_element.r#type
                && cached_element.overpass_data.id == fresh_overpass_element.id
        });
        if cached_element.is_none() {
            continue;
        }
        let mut cached_element = cached_element.unwrap().clone();
        if cached_element.deleted_at.is_some() {
            cached_element = cached_element.set_deleted_at(None, conn)?;
        }
        if *fresh_overpass_element == cached_element.overpass_data {
            continue;
        }
        user::service::insert_user_if_not_exists(fresh_overpass_element.uid.unwrap(), conn).await?;
        let sp = conn.savepoint()?;
        if fresh_overpass_element.changeset != cached_element.overpass_data.changeset {
            let event = Event::insert(
                fresh_overpass_element.uid.unwrap(),
                cached_element.id,
                "update",
                &sp,
            )?;
            res.push(event);
        }
        let mut updated_element = cached_element.set_overpass_data(fresh_overpass_element, &sp)?;
        let new_android_icon = updated_element.overpass_data.generate_android_icon();
        let old_android_icon = cached_element
            .tag("icon:android")
            .as_str()
            .unwrap_or_default();
        if new_android_icon != old_android_icon {
            updated_element = Element::set_tag(
                updated_element.id,
                "icon:android",
                &new_android_icon.clone().into(),
                &sp,
            )?;
        }
        element::service::generate_issues(vec![&updated_element], &sp)?;
        //area_element::service::generate_mapping(&vec![updated_element], &sp)?;
        sp.commit()?;
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
                user::service::insert_user_if_not_exists(user_id.unwrap(), conn).await?;
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
                element::service::generate_issues(vec![&element], &sp)?;
                //area_element::service::generate_mapping(&vec![element], &sp)?;
                sp.commit()?;
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::{
        conf::Conf,
        element::Element,
        osm::{api::OsmUser, overpass::OverpassElement},
        test::mock_conn,
        user::{self, User},
        Result,
    };
    use actix_web::test;

    #[test]
    #[ignore = "relies on external service"]
    async fn sync_deleted_elements() -> Result<()> {
        let mut conn = mock_conn();
        let element_1 = Element::insert(&OverpassElement::mock(1), &conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &conn)?;
        let element_3 = Element::insert(&OverpassElement::mock(2702291726), &conn)?;
        let res = super::sync_deleted_elements(
            &vec![element_1.overpass_data, element_2.overpass_data],
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
        assert!(super::confirm_deleted("node", 2702291726, &Conf::mock())
            .await
            .is_ok());
        assert!(super::confirm_deleted("node", 12181429828, &Conf::mock())
            .await
            .is_err());
        Ok(())
    }

    #[test]
    async fn insert_user_if_not_exists_when_cached() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        assert!(user::service::insert_user_if_not_exists(user.id, &conn)
            .await
            .is_ok());
        Ok(())
    }

    #[test]
    #[ignore = "relies on external service"]
    async fn insert_user_if_not_exists_when_exists_on_osm() -> Result<()> {
        let conn = mock_conn();
        let btc_map_user_id = 18545877;
        assert!(
            user::service::insert_user_if_not_exists(btc_map_user_id, &conn)
                .await
                .is_ok()
        );
        assert!(User::select_by_id(btc_map_user_id, &conn)?.is_some());
        Ok(())
    }
}
