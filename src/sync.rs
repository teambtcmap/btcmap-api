use crate::area_element::service::Diff;
use crate::conf::Conf;
use crate::element::{self, Element};
use crate::element_issue::model::ElementIssue;
use crate::event::{self, Event};
use crate::osm::overpass::OverpassElement;
use crate::osm::{self, api::OsmElement};
use crate::{area_element, discord, user, Error, Result};
use deadpool_sqlite::Pool;
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
    pool: &Pool,
) -> Result<MergeResult> {
    let started_at = OffsetDateTime::now_utc();
    // stage 1: find and process deleted elements
    let deleted_element_events = sync_deleted_elements(&fresh_overpass_elements, pool).await?;
    let mut deleted_elements: Vec<MergeResultElement> = vec![];
    for event in &deleted_element_events {
        let element = Element::select_by_id_async(event.element_id, pool).await?;
        deleted_elements.push(element.into());
    }
    let deleted_sync_time_s = (OffsetDateTime::now_utc() - started_at).as_seconds_f64();

    // stage 2: find and process updated elements
    let updated_sync_started_at = OffsetDateTime::now_utc();
    let updated_element_events = sync_updated_elements(&fresh_overpass_elements, pool).await?;
    let mut updated_elements: Vec<Element> = vec![];
    for event in &updated_element_events {
        let element = Element::select_by_id_async(event.element_id, pool).await?;
        updated_elements.push(element);
    }
    let updated_sync_time_s =
        (OffsetDateTime::now_utc() - updated_sync_started_at).as_seconds_f64();

    // stage 3: find and process new elements
    let created_sync_started_at = OffsetDateTime::now_utc();
    let created_element_events = sync_new_elements(&fresh_overpass_elements, pool).await?;
    let mut created_elements: Vec<Element> = vec![];
    for event in &created_element_events {
        let element = Element::select_by_id_async(event.element_id, pool).await?;
        created_elements.push(element);
    }
    let created_sync_time_s =
        (OffsetDateTime::now_utc() - created_sync_started_at).as_seconds_f64();

    let events_processing_started_at = OffsetDateTime::now_utc();
    let mut all_events: Vec<Event> = vec![];
    all_events.extend(created_element_events);
    all_events.extend(updated_element_events);
    all_events.extend(deleted_element_events);
    for event in all_events {
        event::service::on_new_event(&event, pool).await?;
    }
    let events_processing_time_s =
        (OffsetDateTime::now_utc() - events_processing_started_at).as_seconds_f64();

    let area_mapping_started_at = OffsetDateTime::now_utc();
    let mut area_mapping_elements: Vec<Element> = vec![];
    area_mapping_elements.extend(created_elements.clone());
    area_mapping_elements.extend(updated_elements.clone());
    let area_mapping_diff =
        area_element::service::generate_mapping(&area_mapping_elements, pool).await?;
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
    fresh_overpass_elements: &[OverpassElement],
    pool: &Pool,
) -> Result<Vec<Event>> {
    let fresh_overpass_element_ids: HashSet<String> = fresh_overpass_elements
        .iter()
        .map(|it| it.btcmap_id())
        .collect();
    let absent_elements: Vec<Element> = Element::select_all_except_deleted_async(pool)
        .await?
        .into_iter()
        .filter(|it| !fresh_overpass_element_ids.contains(&it.overpass_data.btcmap_id()))
        .collect();
    let mut res = vec![];
    let conf = Conf::select_async(pool).await?;
    for absent_element in absent_elements {
        let fresh_osm_element = confirm_deleted(
            &absent_element.overpass_data.r#type,
            absent_element.overpass_data.id,
            &conf,
        )
        .await?;
        user::service::insert_user_if_not_exists(fresh_osm_element.uid, pool).await?;
        res.push(mark_element_as_deleted(&absent_element, &fresh_osm_element, pool).await?);
    }
    Ok(res)
}

async fn mark_element_as_deleted(
    element: &Element,
    fresh_osm_element: &OsmElement,
    pool: &Pool,
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
    Element::set_deleted_at_async(element.id, Some(OffsetDateTime::now_utc()), pool).await?;
    let element_issues = ElementIssue::select_by_element_id_async(element.id, pool).await?;
    for issue in element_issues {
        ElementIssue::set_deleted_at_async(issue.id, Some(OffsetDateTime::now_utc()), pool).await?;
    }
    let event = Event::insert_async(fresh_osm_element.uid, element.id, "delete", pool).await?;
    let event = Event::patch_tags_async(event.id, event_tags, pool).await?;
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
    pool: &Pool,
) -> Result<Vec<Event>> {
    let mut res = vec![];
    let cached_elements = Element::select_all_async(None, pool).await?;
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
            cached_element = Element::set_deleted_at_async(cached_element.id, None, pool).await?;
        }
        if *fresh_overpass_element == cached_element.overpass_data {
            continue;
        }
        user::service::insert_user_if_not_exists(fresh_overpass_element.uid.unwrap(), pool).await?;
        if fresh_overpass_element.changeset != cached_element.overpass_data.changeset {
            let event = Event::insert_async(
                fresh_overpass_element.uid.unwrap(),
                cached_element.id,
                "update",
                pool,
            )
            .await?;
            res.push(event);
        }
        let mut updated_element = Element::set_overpass_data_async(
            cached_element.id,
            fresh_overpass_element.clone(),
            pool,
        )
        .await?;
        let new_android_icon = updated_element.overpass_data.generate_android_icon();
        let old_android_icon = cached_element
            .tag("icon:android")
            .as_str()
            .unwrap_or_default();
        if new_android_icon != old_android_icon {
            updated_element = Element::set_tag_async(
                updated_element.id,
                "icon:android",
                &new_android_icon.clone().into(),
                pool,
            )
            .await?;
        }
        element::service::generate_issues_async(vec![updated_element], pool).await?;
        //area_element::service::generate_mapping(&vec![updated_element], &sp)?;
    }
    Ok(res)
}

pub async fn sync_new_elements(
    fresh_overpass_elements: &Vec<OverpassElement>,
    pool: &Pool,
) -> Result<Vec<Event>> {
    let mut res = vec![];
    let cached_elements = Element::select_all_async(None, pool).await?;
    for fresh_element in fresh_overpass_elements {
        let btcmap_id = fresh_element.btcmap_id();
        let user_id = fresh_element.uid;

        match cached_elements
            .iter()
            .find(|it| it.overpass_data.btcmap_id() == btcmap_id)
        {
            Some(_) => {}
            None => {
                user::service::insert_user_if_not_exists(user_id.unwrap(), pool).await?;
                let element = Element::insert_async(fresh_element.clone(), pool).await?;
                let event =
                    Event::insert_async(user_id.unwrap(), element.id, "create", pool).await?;
                res.push(event);
                let category = element.overpass_data.generate_category();
                let android_icon = element.overpass_data.generate_android_icon();
                let element =
                    Element::set_tag_async(element.id, "category", &category.clone().into(), pool)
                        .await?;
                let element = Element::set_tag_async(
                    element.id,
                    "icon:android",
                    &android_icon.clone().into(),
                    pool,
                )
                .await?;
                element::service::generate_issues_async(vec![element], pool).await?;
                //area_element::service::generate_mapping(&vec![element], &sp)?;
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::{
        conf::Conf,
        db,
        element::Element,
        osm::{api::EditingApiUser, overpass::OverpassElement},
        test::mock_db,
        user::{self},
        Result,
    };
    use actix_web::test;

    #[test]
    #[ignore = "relies on external service"]
    async fn sync_deleted_elements() -> Result<()> {
        let db = mock_db();
        let element_1 = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &db.conn)?;
        let element_3 = Element::insert(&OverpassElement::mock(2702291726), &db.conn)?;
        let res = super::sync_deleted_elements(
            &vec![element_1.overpass_data, element_2.overpass_data],
            &db.pool,
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
        let db = mock_db();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &db.conn)?;
        assert!(user::service::insert_user_if_not_exists(user.id, &db.pool)
            .await
            .is_ok());
        Ok(())
    }

    #[test]
    #[ignore = "relies on external service"]
    async fn insert_user_if_not_exists_when_exists_on_osm() -> Result<()> {
        let db = mock_db();
        let btc_map_user_id = 18545877;
        assert!(
            user::service::insert_user_if_not_exists(btc_map_user_id, &db.pool)
                .await
                .is_ok()
        );
        assert!(db::osm_user::queries::select_by_id(btc_map_user_id, &db.conn)?.is_some());
        Ok(())
    }
}
