use super::Area;
use crate::{
    area_element::{self, model::AreaElement},
    element::Element,
    element_comment::ElementComment,
    event::Event,
    Error, Result,
};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

pub async fn insert_async(tags: Map<String, Value>, pool: &Pool) -> Result<Area> {
    pool.get()
        .await?
        .interact(move |conn| insert(tags, conn))
        .await?
}

// it can take a long time to find area_elements
// let's say it takes 10 minutes
// on 7th minute a new element was added by osm sync
// this new area will be mapped to that place by the sync
// so we can't assume that insert_bulk will always append to an empty dataset
// but confilct seems unlikely since the place was added after get_elements_within_geometries queried elements snapshot
// and if an element was deleted, that's not an issue since we never fully delete elements
// but wat if an element was moved? It could change its area set... TODO
pub fn insert(tags: Map<String, Value>, conn: &mut Connection) -> Result<Area> {
    let area = Area::insert(tags, conn)?;
    let area_elements =
        area_element::service::get_elements_within_geometries(area.geo_json_geometries()?, conn)?;
    AreaElement::insert_bulk(
        area.id,
        area_elements.into_iter().map(|it| it.id).collect(),
        conn,
    )?;
    Ok(area)
}

pub fn patch_tags(
    area_id_or_alias: &str,
    tags: Map<String, Value>,
    conn: &Connection,
) -> Result<Area> {
    if tags.contains_key("url_alias") {
        return Err(Error::InvalidInput("url_alias can't be changed".into()));
    }
    let area = Area::select_by_id_or_alias(area_id_or_alias, conn)?;
    if tags.contains_key("geo_json") {
        let mut affected_elements: HashSet<Element> = HashSet::new();
        for area_element in AreaElement::select_by_area_id(area.id, conn)? {
            let element = Element::select_by_id(area_element.element_id, conn)?.ok_or(format!(
                "failed to fetch element {}",
                area_element.element_id,
            ))?;
            affected_elements.insert(element);
        }
        let area = Area::patch_tags(area.id, tags, conn)?;
        let elements_in_new_bounds = area_element::service::get_elements_within_geometries(
            area.geo_json_geometries()?,
            conn,
        )?;
        for element in elements_in_new_bounds {
            affected_elements.insert(element);
        }
        let affected_elements: Vec<Element> = affected_elements.into_iter().collect();
        area_element::service::generate_mapping(&affected_elements, conn)?;
        Ok(area)
    } else {
        Area::patch_tags(area.id, tags, conn)
    }
}

pub fn remove_tag(area_id_or_alias: &str, tag_name: &str, conn: &mut Connection) -> Result<Area> {
    if tag_name == "url_alias" {
        return Err(Error::InvalidInput("url_alias can't be removed".into()));
    }
    if tag_name == "geo_json" {
        return Err(Error::InvalidInput("geo_json can't be removed".into()));
    }
    let area = Area::select_by_id_or_alias(area_id_or_alias, conn)?;
    Area::remove_tag(area.id, tag_name, conn)
}

pub fn soft_delete(area_id_or_alias: &str, conn: &Connection) -> Result<Area> {
    let area = Area::select_by_id_or_alias(area_id_or_alias, conn)?;
    let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), conn)?;
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
    let events = Event::select_created_between(period_start, period_end, conn)?;
    let mut areas_to_events: HashMap<i64, Vec<&Event>> = HashMap::new();
    for event in &events {
        let element = Element::select_by_id(event.element_id, conn)?.unwrap();
        let element_area_ids: Vec<i64> = AreaElement::select_by_element_id(element.id, conn)?
            .into_iter()
            .map(|it| it.area_id)
            .collect();
        for element_area_id in element_area_ids {
            areas_to_events.entry(element_area_id).or_default();
            let area_events = areas_to_events.get_mut(&element_area_id).unwrap();
            area_events.push(event);
        }
    }
    let comments = ElementComment::select_created_between(period_start, period_end, conn)?;
    let mut areas_to_comments: HashMap<i64, Vec<&ElementComment>> = HashMap::new();
    for comment in &comments {
        let element = Element::select_by_id(comment.element_id, conn)?.unwrap();
        let element_area_ids: Vec<i64> = AreaElement::select_by_element_id(element.id, conn)?
            .into_iter()
            .map(|it| it.area_id)
            .collect();
        for element_area_id in element_area_ids {
            areas_to_comments.entry(element_area_id).or_default();
            let area_comments = areas_to_comments.get_mut(&element_area_id).unwrap();
            area_comments.push(comment);
        }
    }
    let mut areas: HashSet<i64> = HashSet::new();
    for area_id in areas_to_events.keys() {
        areas.insert(*area_id);
    }
    for area_id in areas_to_comments.keys() {
        areas.insert(*area_id);
    }
    let areas: Vec<_> = areas
        .into_iter()
        .map(|it| Area::select_by_id(it, conn).unwrap())
        .collect();
    let mut res: Vec<TrendingArea> = areas
        .into_iter()
        .filter(|it| it.tags.contains_key("type") && it.tags["type"].as_str() == Some(r#type))
        .map(|it| {
            areas_to_events.entry(it.id).or_default();
            let events = areas_to_events.get(&it.id).unwrap();
            let mut created: Vec<&Event> = vec![];
            let mut updated: Vec<&Event> = vec![];
            let mut deleted: Vec<&Event> = vec![];
            for event in events {
                match event.r#type.as_str() {
                    "create" => created.push(event),
                    "update" => updated.push(event),
                    "delete" => deleted.push(event),
                    _ => {}
                }
            }
            areas_to_comments.entry(it.id).or_default();
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

pub fn get_comments(area: &Area, conn: &Connection) -> Result<Vec<ElementComment>> {
    let area_elements = AreaElement::select_by_area_id(area.id, conn)?;
    let mut comments: Vec<ElementComment> = vec![];
    for area_element in area_elements {
        for comment in ElementComment::select_by_element_id(area_element.element_id, conn)? {
            comments.push(comment);
        }
    }
    Ok(comments)
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::area_element::model::AreaElement;
    use crate::element::Element;
    use crate::element_comment::ElementComment;
    use crate::osm::overpass::OverpassElement;
    use crate::test::{earth_geo_json, mock_conn, phuket_geo_json};
    use crate::Result;
    use serde_json::{json, Map};

    #[test]
    fn insert() -> Result<()> {
        let mut conn = mock_conn();
        let area = super::insert(Area::mock_tags(), &mut conn)?;
        assert_eq!(area, Area::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn insert_should_create_area_mappings() -> Result<()> {
        let mut conn = mock_conn();
        let element_1 = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        Element::insert(&element_1, &conn)?;
        let element_2 = OverpassElement {
            lat: Some(50.0),
            lon: Some(1.0),
            ..OverpassElement::mock(2)
        };
        Element::insert(&element_2, &conn)?;
        let mut tags = Area::mock_tags();
        tags.insert("geo_json".into(), phuket_geo_json());
        super::insert(tags, &mut conn)?;
        assert_eq!(1, AreaElement::select_by_area_id(1, &conn)?.len());
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let mut conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &mut conn)?;
        let mut patch_set = Map::new();
        let new_tag_name = "foo";
        let new_tag_value = json!("bar");
        patch_set.insert(new_tag_name.into(), new_tag_value.clone());
        let area = super::patch_tags(&area.id.to_string(), patch_set, &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?;
        assert_eq!(area, db_area);
        assert_eq!(new_tag_value, db_area.tags[new_tag_name]);
        Ok(())
    }

    #[test]
    fn patch_tags_should_update_area_mappings() -> Result<()> {
        let mut conn = mock_conn();
        let element_in_phuket = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        Element::insert(&element_in_phuket, &conn)?;
        let element_in_london = OverpassElement {
            lat: Some(50.0),
            lon: Some(1.0),
            ..OverpassElement::mock(2)
        };
        Element::insert(&element_in_london, &conn)?;
        let mut tags = Area::mock_tags();
        tags.insert("geo_json".into(), earth_geo_json());
        let area = Area::insert(tags.clone(), &mut conn)?;
        let area_element_phuket = AreaElement::insert(area.id, element_in_phuket.id, &conn)?;
        let area_element_london = AreaElement::insert(area.id, element_in_london.id, &conn)?;
        tags.insert("geo_json".into(), phuket_geo_json());
        tags.remove("url_alias");
        let area = super::patch_tags(&area.id.to_string(), tags.clone(), &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?;
        assert_eq!(area, db_area);
        assert!(AreaElement::select_by_id(area_element_phuket.id, &conn)?
            .ok_or("failed to insert area element")?
            .deleted_at
            .is_none());
        assert!(AreaElement::select_by_id(area_element_london.id, &conn)?
            .ok_or("failed to insert area element")?
            .deleted_at
            .is_some());
        assert_eq!(2, AreaElement::select_by_area_id(area.id, &conn)?.len());
        tags.insert("geo_json".into(), earth_geo_json());
        let area = super::patch_tags(&area.id.to_string(), tags, &mut conn)?;
        assert_eq!(2, AreaElement::select_by_area_id(area.id, &conn)?.len());
        assert!(AreaElement::select_by_id(area_element_phuket.id, &conn)?
            .ok_or("failed to insert area element")?
            .deleted_at
            .is_none());
        assert!(AreaElement::select_by_id(area_element_london.id, &conn)?
            .ok_or("failed to insert area element")?
            .deleted_at
            .is_none());
        Ok(())
    }

    #[test]
    fn soft_delete() -> Result<()> {
        let mut conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        super::soft_delete(&area.id.to_string(), &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?;
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
        let area = Area::insert(Area::mock_tags(), &mut conn)?;
        super::soft_delete(&area.id.to_string(), &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?;
        assert!(db_area.deleted_at.is_some());
        assert!(db_area.tags.get("areas").is_none());
        Ok(())
    }

    #[test]
    fn get_comments() -> Result<()> {
        let conn = mock_conn();
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let comment = ElementComment::insert(element.id, "test", &conn)?;
        let area = Area::insert(Area::mock_tags(), &conn)?;
        let _area_element = AreaElement::insert(area.id, element.id, &conn)?;
        assert_eq!(Some(&comment), super::get_comments(&area, &conn)?.first());
        Ok(())
    }
}
