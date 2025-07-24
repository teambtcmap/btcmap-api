use crate::db::event::schema::Event;
use crate::service;
use crate::{
    db::{
        self, area::schema::Area, element::schema::Element, element_comment::schema::ElementComment,
    },
    Result,
};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

// it can take a long time to find area_elements
// let's say it takes 10 minutes
// on 7th minute a new element was added by osm sync
// this new area will be mapped to that place by the sync
// so we can't assume that insert_bulk will always append to an empty dataset
// but confilct seems unlikely since the place was added after get_elements_within_geometries queried elements snapshot
// and if an element was deleted, that's not an issue since we never fully delete elements
// but wat if an element was moved? It could change its area set... TODO
pub async fn insert(tags: Map<String, Value>, pool: &Pool) -> Result<Area> {
    let area = db::area::queries_async::insert(tags, pool).await?;
    let area_elements = service::area_element::get_elements_within_geometries_async(
        area.geo_json_geometries()?,
        pool,
    )
    .await?;
    for element in area_elements {
        db::area_element::queries_async::insert(area.id, element.id, pool).await?;
    }
    Ok(area)
}

pub async fn patch_tags(
    area_id_or_alias: &str,
    tags: Map<String, Value>,
    pool: &Pool,
) -> Result<Area> {
    if tags.contains_key("url_alias") {
        return Err("url_alias can't be changed".into());
    }
    let area = db::area::queries_async::select_by_id_or_alias(area_id_or_alias, pool).await?;
    if tags.contains_key("geo_json") {
        let mut affected_element_ids: HashSet<i64> = HashSet::new();
        for area_element in
            db::area_element::queries_async::select_by_area_id(area.id, pool).await?
        {
            let element =
                db::element::queries_async::select_by_id(area_element.element_id, pool).await?;
            affected_element_ids.insert(element.id);
        }
        let area = db::area::queries_async::patch_tags(area.id, tags, pool).await?;
        let elements_in_new_bounds = service::area_element::get_elements_within_geometries_async(
            area.geo_json_geometries()?,
            pool,
        )
        .await?;
        for element in elements_in_new_bounds {
            affected_element_ids.insert(element.id);
        }
        let mut affected_elements: Vec<Element> = vec![];
        for id in affected_element_ids {
            affected_elements.push(db::element::queries_async::select_by_id(id, &pool).await?);
        }
        service::area_element::generate_mapping(&affected_elements, pool).await?;
        Ok(area)
    } else {
        db::area::queries_async::patch_tags(area.id, tags, pool).await
    }
}

pub async fn remove_tag_async(
    area_id_or_alias: impl Into<String>,
    tag_name: impl Into<String>,
    pool: &Pool,
) -> Result<Area> {
    let area_id_or_alias = area_id_or_alias.into();
    let tag_name = tag_name.into();

    if tag_name == "url_alias" {
        return Err("url_alias can't be removed".into());
    }
    if tag_name == "geo_json" {
        return Err("geo_json can't be removed".into());
    }
    let area = db::area::queries_async::select_by_id_or_alias(area_id_or_alias, pool).await?;
    db::area::queries_async::remove_tag(area.id, tag_name, pool).await
}

pub async fn soft_delete_async(area_id_or_alias: impl Into<String>, pool: &Pool) -> Result<Area> {
    let area_id_or_alias = area_id_or_alias.into();
    let area = db::area::queries_async::select_by_id_or_alias(area_id_or_alias, pool).await?;
    db::area::queries_async::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), pool).await
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

pub async fn get_trending_areas_async(
    r#type: impl Into<String>,
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    pool: &Pool,
) -> Result<Vec<TrendingArea>> {
    let r#type = r#type.into();
    let period_start = *period_start;
    let period_end = *period_end;
    get_trending_areas(&r#type, period_start, period_end, pool).await
}

pub async fn get_trending_areas(
    r#type: &str,
    period_start: OffsetDateTime,
    period_end: OffsetDateTime,
    pool: &Pool,
) -> Result<Vec<TrendingArea>> {
    let events =
        db::event::queries_async::select_created_between(period_start, period_end, pool).await?;
    let mut areas_to_events: HashMap<i64, Vec<&Event>> = HashMap::new();
    for event in &events {
        let element = db::element::queries_async::select_by_id(event.element_id, pool).await?;
        let element_area_ids: Vec<i64> =
            db::area_element::queries_async::select_by_element_id(element.id, pool)
                .await?
                .into_iter()
                .map(|it| it.area_id)
                .collect();
        for element_area_id in element_area_ids {
            areas_to_events.entry(element_area_id).or_default();
            let area_events = areas_to_events.get_mut(&element_area_id).unwrap();
            area_events.push(event);
        }
    }
    let comments =
        db::element_comment::queries_async::select_created_between(period_start, period_end, pool)
            .await?;
    let comments: Vec<ElementComment> = comments
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    let mut areas_to_comments: HashMap<i64, Vec<&ElementComment>> = HashMap::new();
    for comment in &comments {
        let element = db::element::queries_async::select_by_id(comment.element_id, pool).await?;
        let element_area_ids: Vec<i64> =
            db::area_element::queries_async::select_by_element_id(element.id, pool)
                .await?
                .into_iter()
                .map(|it| it.area_id)
                .collect();
        for element_area_id in element_area_ids {
            areas_to_comments.entry(element_area_id).or_default();
            let area_comments = areas_to_comments.get_mut(&element_area_id).unwrap();
            area_comments.push(comment);
        }
    }
    let mut area_ids: HashSet<i64> = HashSet::new();
    for area_id in areas_to_events.keys() {
        area_ids.insert(*area_id);
    }
    for area_id in areas_to_comments.keys() {
        area_ids.insert(*area_id);
    }
    let mut areas: Vec<Area> = vec![];
    for id in area_ids {
        areas.push(db::area::queries_async::select_by_id(id, pool).await?);
    }
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
    Ok(res.into_iter().take(10).collect())
}

pub async fn get_comments_async(
    area: Area,
    include_deleted: bool,
    pool: &Pool,
) -> Result<Vec<ElementComment>> {
    pool.get()
        .await?
        .interact(move |conn| get_comments(&area, include_deleted, conn))
        .await?
}

fn get_comments(
    area: &Area,
    include_deleted: bool,
    conn: &Connection,
) -> Result<Vec<ElementComment>> {
    let area_elements = db::area_element::queries::select_by_area_id(area.id, conn)?;
    let mut comments: Vec<ElementComment> = vec![];
    for area_element in area_elements {
        for comment in db::element_comment::queries::select_by_element_id(
            area_element.element_id,
            include_deleted,
            i64::MAX,
            conn,
        )? {
            comments.push(comment);
        }
    }
    Ok(comments)
}

#[cfg(test)]
mod test {
    use crate::db::area::schema::Area;
    use crate::db::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test;
    use serde_json::{json, Map};

    #[test]
    async fn insert() -> Result<()> {
        let pool = pool();
        let area = super::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(
            area.id,
            db::area::queries_async::select_by_id(1, &pool).await?.id
        );
        Ok(())
    }

    #[test]
    async fn insert_should_create_area_mappings() -> Result<()> {
        let pool = pool();
        let element_1 = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        db::element::queries_async::insert(element_1, &pool).await?;
        let element_2 = OverpassElement {
            lat: Some(50.0),
            lon: Some(1.0),
            ..OverpassElement::mock(2)
        };
        db::element::queries_async::insert(element_2, &pool).await?;
        let mut tags = Area::mock_tags();
        // Phuket
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [98.2181205776469, 8.20412838698085],
                              [98.2181205776469, 7.74024270965898],
                              [98.4806081271079, 7.74024270965898],
                              [98.4806081271079, 8.20412838698085],
                              [98.2181205776469, 8.20412838698085]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        super::insert(tags, &pool).await?;
        assert_eq!(
            1,
            db::area_element::queries_async::select_by_area_id(1, &pool)
                .await?
                .len()
        );
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let pool = pool();
        let area = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let mut patch_set = Map::new();
        let new_tag_name = "foo";
        let new_tag_value = json!("bar");
        patch_set.insert(new_tag_name.into(), new_tag_value.clone());
        let area = super::patch_tags(&area.id.to_string(), patch_set, &pool).await?;
        let db_area = db::area::queries_async::select_by_id(area.id, &pool).await?;
        assert_eq!(area.id, db_area.id);
        assert_eq!(new_tag_value, db_area.tags[new_tag_name]);
        Ok(())
    }

    #[test]
    async fn patch_tags_should_update_area_mappings() -> Result<()> {
        let pool = pool();
        let element_in_phuket = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        db::element::queries_async::insert(element_in_phuket.clone(), &pool).await?;
        let element_in_london = OverpassElement {
            lat: Some(50.0),
            lon: Some(1.0),
            ..OverpassElement::mock(2)
        };
        db::element::queries_async::insert(element_in_london.clone(), &pool).await?;
        let mut tags = Area::mock_tags();
        // Earth
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [-180,-90],
                              [-180,90],
                              [180,90],
                              [180,-90],
                              [-180,-90]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        let area = db::area::queries_async::insert(tags.clone(), &pool).await?;
        let area_element_phuket =
            db::area_element::queries_async::insert(area.id, element_in_phuket.id, &pool).await?;
        let area_element_london =
            db::area_element::queries_async::insert(area.id, element_in_london.id, &pool).await?;
        // Phuket
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [98.2181205776469, 8.20412838698085],
                              [98.2181205776469, 7.74024270965898],
                              [98.4806081271079, 7.74024270965898],
                              [98.4806081271079, 8.20412838698085],
                              [98.2181205776469, 8.20412838698085]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        tags.remove("url_alias");
        let area = super::patch_tags(&area.id.to_string(), tags.clone(), &pool).await?;
        let db_area = db::area::queries_async::select_by_id(area.id, &pool).await?;
        assert_eq!(area.id, db_area.id);
        assert!(
            db::area_element::queries_async::select_by_id(area_element_phuket.id, &pool)
                .await?
                .deleted_at
                .is_none()
        );
        assert!(
            db::area_element::queries_async::select_by_id(area_element_london.id, &pool)
                .await?
                .deleted_at
                .is_some()
        );
        assert_eq!(
            2,
            db::area_element::queries_async::select_by_area_id(area.id, &pool)
                .await?
                .len()
        );
        // Earth
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [-180,-90],
                              [-180,90],
                              [180,90],
                              [180,-90],
                              [-180,-90]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        let area = super::patch_tags(&area.id.to_string(), tags, &pool).await?;
        assert_eq!(
            2,
            db::area_element::queries_async::select_by_area_id(area.id, &pool)
                .await?
                .len()
        );
        assert!(
            db::area_element::queries_async::select_by_id(area_element_phuket.id, &pool)
                .await?
                .deleted_at
                .is_none()
        );
        assert!(
            db::area_element::queries_async::select_by_id(area_element_london.id, &pool)
                .await?
                .deleted_at
                .is_none()
        );
        Ok(())
    }

    #[test]
    async fn soft_delete() -> Result<()> {
        let pool = pool();
        let area = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        super::soft_delete_async(&area.id.to_string(), &pool).await?;
        let db_area = db::area::queries_async::select_by_id(area.id, &pool).await?;
        assert!(db_area.deleted_at.is_some());
        Ok(())
    }

    #[test]
    async fn soft_delete_should_update_areas_tags() -> Result<()> {
        let pool = pool();
        let area_element = OverpassElement {
            lat: Some(7.979623499157051),
            lon: Some(98.33448362485439),
            ..OverpassElement::mock(1)
        };
        let area_element = db::element::queries_async::insert(area_element, &pool).await?;
        db::element::queries_async::set_tag(
            area_element.id,
            "areas",
            &json!("[{id:1},{id:2}]"),
            &pool,
        )
        .await?;
        let area = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        super::soft_delete_async(&area.id.to_string(), &pool).await?;
        let db_area = db::area::queries_async::select_by_id(area.id, &pool).await?;
        assert!(db_area.deleted_at.is_some());
        assert!(db_area.tags.get("areas").is_none());
        Ok(())
    }

    #[test]
    async fn get_comments() -> Result<()> {
        let pool = pool();
        let element = db::element::queries_async::insert(OverpassElement::mock(1), &pool).await?;
        let comment = db::element_comment::queries_async::insert(element.id, "test", &pool).await?;
        let area = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let _area_element =
            db::area_element::queries_async::insert(area.id, element.id, &pool).await?;
        assert_eq!(
            Some(&comment),
            super::get_comments_async(area, false, &pool).await?.first()
        );
        Ok(())
    }
}
