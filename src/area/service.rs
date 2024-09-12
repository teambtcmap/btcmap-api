use super::Area;
use crate::{
    area_element::model::AreaElement,
    element::{self, Element},
    element_comment::ElementComment,
    event::Event,
    Error, Result,
};
use geojson::GeoJson;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

pub fn insert(tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
    if !tags.contains_key("geo_json") {
        return Err(Error::HttpBadRequest("geo_json tag is missing".into()));
    }
    let url_alias = tags
        .get("url_alias")
        .ok_or(Error::HttpBadRequest(
            "Mandatory tag is missing: url_alias".into(),
        ))?
        .as_str()
        .ok_or(Error::HttpBadRequest(
            "This tag should be a string: url_alias".into(),
        ))?;
    let geo_json = tags
        .get("geo_json")
        .ok_or(Error::HttpBadRequest(
            "Mandatory tag is missing: geo_json".into(),
        ))?
        .as_object()
        .ok_or(Error::HttpBadRequest(
            "This tag should be an object: geo_json".into(),
        ))?;
    let geo_json: Result<GeoJson, _> = serde_json::to_string(geo_json).unwrap().parse();
    if geo_json.is_err() {
        Err(Error::HttpConflict("Invalid geo_json".into()))?
    }
    if Area::select_by_alias(url_alias, &conn)?.is_some() {
        Err(Error::HttpConflict(
            "This url_alias is already in use".into(),
        ))?
    }
    let area = Area::insert(geo_json.unwrap(), tags, &conn)?;
    let area_elements = element::service::find_in_area(&area, conn)?;
    element::service::generate_areas_mapping_old(&area_elements, conn)?;
    Ok(area)
}

pub fn patch_tag(
    id_or_alias: &str,
    tag_name: &str,
    tag_value: &Value,
    conn: &mut Connection,
) -> Result<Area> {
    let mut tags = Map::new();
    tags.insert(tag_name.to_string(), tag_value.clone());
    patch_tags(id_or_alias, tags, conn)
}

pub fn patch_tags(id_or_alias: &str, tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
    let area = Area::select_by_id_or_alias(id_or_alias, conn)?.unwrap();
    if tags.contains_key("geo_json") {
        let area_elements = element::service::find_in_area(&area, conn)?;
        let area = Area::patch_tags(area.id, tags, conn)?;
        element::service::generate_areas_mapping_old(&area_elements, conn)?;
        let area_elements = element::service::find_in_area(&area, conn)?;
        element::service::generate_areas_mapping_old(&area_elements, conn)?;
        Ok(area)
    } else {
        Ok(Area::patch_tags(area.id, tags, conn)?)
    }
}

pub fn remove_tag(area_id_or_alias: &str, tag_name: &str, conn: &mut Connection) -> Result<Area> {
    if tag_name == "geo_json" {
        return Err(Error::HttpBadRequest(
            "geo_json tag can't be removed".into(),
        ));
    }
    let area = Area::select_by_id_or_alias(area_id_or_alias, conn)?.unwrap();
    Ok(Area::remove_tag(area.id, tag_name, conn)?)
}

pub fn soft_delete(area_id_or_alias: &str, conn: &Connection) -> Result<Area> {
    let area = Area::select_by_id_or_alias(area_id_or_alias, conn)?.unwrap();
    let area_elements = element::service::find_in_area(&area, conn)?;
    let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), conn)?;
    element::service::generate_areas_mapping_old(&area_elements, conn)?;
    Ok(area)
}

#[derive(Serialize)]
pub struct TrendingArea {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub events: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: i64,
    pub comments: i64,
}

pub fn get_trending_areas(
    r#type: &str,
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<TrendingArea>> {
    let events = Event::select_created_between(&period_start, &period_end, &conn)?;
    let mut areas_to_events: HashMap<i64, Vec<&Event>> = HashMap::new();
    for event in &events {
        let element = Element::select_by_id(event.element_id, &conn)?.unwrap();
        let element_area_ids: Vec<i64> = AreaElement::select_by_element_id(element.id, conn)?
            .into_iter()
            .map(|it| it.area_id)
            .collect();
        for element_area_id in element_area_ids {
            if !areas_to_events.contains_key(&element_area_id) {
                areas_to_events.insert(element_area_id, vec![]);
            }
            let area_events = areas_to_events.get_mut(&element_area_id).unwrap();
            area_events.push(event);
        }
    }
    let comments = ElementComment::select_created_between(&period_start, &period_end, &conn)?;
    let mut areas_to_comments: HashMap<i64, Vec<&ElementComment>> = HashMap::new();
    for comment in &comments {
        let element = Element::select_by_id(comment.element_id, &conn)?.unwrap();
        let element_area_ids: Vec<i64> = AreaElement::select_by_element_id(element.id, conn)?
            .into_iter()
            .map(|it| it.area_id)
            .collect();
        for element_area_id in element_area_ids {
            if !areas_to_comments.contains_key(&element_area_id) {
                areas_to_comments.insert(element_area_id, vec![]);
            }
            let area_comments = areas_to_comments.get_mut(&element_area_id).unwrap();
            area_comments.push(comment);
        }
    }
    let mut areas: HashSet<i64> = HashSet::new();
    for (area_id, _) in &areas_to_events {
        areas.insert(*area_id);
    }
    for (area_id, _) in &areas_to_comments {
        areas.insert(*area_id);
    }
    let areas: Vec<_> = areas
        .into_iter()
        .map(|it| Area::select_by_id(it, &conn).unwrap().unwrap())
        .collect();
    let mut res: Vec<TrendingArea> = areas
        .into_iter()
        .filter(|it| it.tags.contains_key("type") && it.tags["type"].as_str() == Some(r#type))
        .map(|it| {
            if !areas_to_events.contains_key(&it.id) {
                areas_to_events.insert(it.id, vec![]);
            }
            let events = areas_to_events.get(&it.id).unwrap();
            let mut created: Vec<&Event> = vec![];
            let mut updated: Vec<&Event> = vec![];
            let mut deleted: Vec<&Event> = vec![];
            for event in events {
                match event.r#type.as_str() {
                    "create" => created.push(&event),
                    "update" => updated.push(&event),
                    "delete" => deleted.push(&event),
                    _ => {}
                }
            }
            if !areas_to_comments.contains_key(&it.id) {
                areas_to_comments.insert(it.id, vec![]);
            }
            let comments = areas_to_comments.get(&it.id).unwrap();
            TrendingArea {
                id: it.id,
                name: it.name(),
                url: format!("https://btcmap.org/{}/{}", r#type, it.alias()),
                events: events.len() as i64,
                created: created.len() as i64,
                updated: updated.len() as i64,
                deleted: deleted.len() as i64,
                comments: comments.len() as i64,
            }
        })
        .filter(|it| it.events + it.comments != 0)
        .collect();
    res.sort_by(|a, b| {
        (b.created + b.updated + b.deleted + b.comments)
            .cmp(&(a.created + a.updated + a.deleted + a.comments))
    });
    Ok(res)
}

// fn get_comments(area: &Area, conn: &Connection) -> Result<Vec<ElementComment>> {
//     let area_elements = AreaElement::select_by_area_id(area.id, conn)?;
//     let mut comments: Vec<ElementComment> = vec![];
//     for area_element in area_elements {
//         for comment in ElementComment::select_by_element_id(area_element.element_id, conn)? {
//             comments.push(comment);
//         }
//     }
//     Ok(comments)
// }

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::{mock_conn, mock_tags, phuket_geo_json};
    use crate::Result;
    use geojson::{Feature, GeoJson};
    use serde_json::{json, Map};

    #[test]
    fn insert() -> Result<()> {
        let mut conn = mock_conn();
        let mut tags = mock_tags();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        tags.insert("geo_json".into(), phuket_geo_json());
        let area = super::insert(tags, &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert_eq!(area, db_area);
        Ok(())
    }

    #[test]
    fn insert_should_update_areas_tags() -> Result<()> {
        let mut conn = mock_conn();
        let area_element = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        let area_element = Element::insert(&area_element, &conn)?;
        let mut tags = mock_tags();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        tags.insert("geo_json".into(), phuket_geo_json());
        super::insert(tags, &mut conn)?;
        let db_area_element = Element::select_by_id(area_element.id, &conn)?.unwrap();
        assert_eq!(1, db_area_element.tag("areas").as_array().unwrap().len());
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let mut conn = mock_conn();
        let mut tags = Map::new();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        let area = Area::insert(
            GeoJson::from_json_value(phuket_geo_json()).unwrap(),
            tags,
            &mut conn,
        )?;
        let mut patch_tags = Map::new();
        let new_tag_name = "foo";
        let new_tag_value = json!("bar");
        patch_tags.insert(new_tag_name.into(), new_tag_value.clone());
        let new_alias = json!("test1");
        patch_tags.insert("url_alias".into(), new_alias.clone());
        let area = super::patch_tags(&area.id.to_string(), patch_tags, &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert_eq!(area, db_area);
        assert_eq!(new_tag_value, db_area.tags[new_tag_name]);
        assert_eq!(new_alias, db_area.tags["url_alias"]);
        Ok(())
    }

    #[test]
    fn patch_tags_should_update_areas_tags() -> Result<()> {
        let mut conn = mock_conn();
        let area_element = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        let area_element = Element::insert(&area_element, &conn)?;
        let area_element =
            Element::set_tag(area_element.id, "areas", &json!("[{id:1},{id:2}]"), &conn)?;
        let mut tags = Map::new();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        tags.insert("geo_json".into(), phuket_geo_json());
        let area = Area::insert(
            GeoJson::from_json_value(phuket_geo_json()).unwrap(),
            tags.clone(),
            &mut conn,
        )?;
        let area = super::patch_tags(&area.id.to_string(), tags, &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert_eq!(area, db_area);
        let db_area_element = Element::select_by_id(area_element.id, &conn)?.unwrap();
        assert_eq!(1, db_area_element.tag("areas").as_array().unwrap().len());
        Ok(())
    }

    #[test]
    fn soft_delete() -> Result<()> {
        let mut conn = mock_conn();
        let area = Area::insert(GeoJson::Feature(Feature::default()), Map::new(), &conn)?;
        super::soft_delete(&area.id.to_string(), &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert!(db_area.deleted_at.is_some());
        Ok(())
    }

    #[test]
    fn soft_delete_should_update_areas_tags() -> Result<()> {
        let mut conn = mock_conn();
        let area_element = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        let area_element = Element::insert(&area_element, &conn)?;
        Element::set_tag(area_element.id, "areas", &json!("[{id:1},{id:2}]"), &conn)?;
        let mut tags = Map::new();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        let area = Area::insert(
            GeoJson::from_json_value(phuket_geo_json()).unwrap(),
            tags,
            &mut conn,
        )?;
        super::soft_delete(&area.id.to_string(), &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert!(db_area.deleted_at.is_some());
        assert!(db_area.tags.get("areas").is_none());
        Ok(())
    }
}
