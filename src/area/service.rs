use super::Area;
use crate::{
    element::{self},
    Error, Result,
};
use geojson::GeoJson;
use rusqlite::Connection;
use serde_json::{Map, Value};
use time::OffsetDateTime;

pub fn insert(tags: Map<String, Value>, conn: &mut Connection) -> Result<Area> {
    if !tags.contains_key("geo_json") {
        return Err(Error::HttpBadRequest("geo_json tag is missing".into()));
    }
    let sp = conn.savepoint()?;
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
    if Area::select_by_alias(url_alias, &sp)?.is_some() {
        Err(Error::HttpConflict(
            "This url_alias is already in use".into(),
        ))?
    }
    let area = Area::insert(tags, &sp)?;
    let area_elements = element::service::find_in_area(&area, &sp)?;
    element::service::update_areas_tag(&area_elements, &sp)?;
    sp.commit()?;
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

pub fn patch_tags(
    id_or_alias: &str,
    tags: Map<String, Value>,
    conn: &mut Connection,
) -> Result<Area> {
    let area = Area::select_by_id_or_alias(id_or_alias, conn)?.unwrap();
    if tags.contains_key("geo_json") {
        let sp = conn.savepoint()?;
        let area_elements = element::service::find_in_area(&area, &sp)?;
        element::service::update_areas_tag(&area_elements, &sp)?;
        let area = Area::patch_tags(area.id, tags, &sp)?;
        let area_elements = element::service::find_in_area(&area, &sp)?;
        element::service::update_areas_tag(&area_elements, &sp)?;
        sp.commit()?;
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

pub fn soft_delete(id: i64, conn: &mut Connection) -> Result<Area> {
    let sp = conn.savepoint()?;
    let area = Area::select_by_id(id, &sp)?.unwrap();
    let area_elements = element::service::find_in_area(&area, &sp)?;
    let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &sp)?;
    element::service::update_areas_tag(&area_elements, &sp)?;
    sp.commit()?;
    Ok(area)
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::{mock_conn, mock_tags, phuket_geo_json};
    use crate::Result;
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
        tags.insert("geo_json".into(), phuket_geo_json());
        let area = Area::insert(tags, &mut conn)?;
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
        let area = Area::insert(tags.clone(), &mut conn)?;
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
        let area = Area::insert(Map::new(), &conn)?;
        super::soft_delete(area.id, &mut conn)?;
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
        tags.insert("geo_json".into(), phuket_geo_json());
        let area = Area::insert(tags, &mut conn)?;
        super::soft_delete(area.id, &mut conn)?;
        let db_area = Area::select_by_id(area.id, &conn)?.unwrap();
        assert!(db_area.deleted_at.is_some());
        assert!(db_area.tags.get("areas").is_none());
        Ok(())
    }
}
