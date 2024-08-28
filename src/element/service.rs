use super::Element;
use crate::area::Area;
use crate::Result;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use time::macros::format_description;
use time::Date;
use tracing::info;

pub fn remove_areas_tag(area: &Area, conn: &mut Connection) -> Result<()> {
    let sp = conn.savepoint()?;
    info!(
        area.id,
        alias = area.alias(),
        "Removing areas tag from area {} ({})",
        area.id,
        area.name(),
    );
    let area_elements = find_in_area(area, &sp)?;
    info!(
        count = area_elements.len(),
        "Found {} elements in {}",
        area_elements.len(),
        area.name(),
    );
    for area_element in area_elements {
        Element::remove_tag(area_element.id, "areas", &sp)?;
    }
    sp.commit()?;
    Ok(())
}

pub fn find_in_area(area: &Area, conn: &Connection) -> Result<Vec<Element>> {
    let all_elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();
    filter_by_area(&all_elements, area)
}

pub fn filter_by_area(all_elements: &Vec<Element>, area: &Area) -> Result<Vec<Element>> {
    let geometries = area.geo_json_geometries();
    let mut area_elements: Vec<Element> = vec![];

    for element in all_elements {
        for geometry in &geometries {
            match &geometry.value {
                geojson::Value::MultiPolygon(_) => {
                    let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                    if multi_poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::Value::Polygon(_) => {
                    let poly: Polygon = (&geometry.value).try_into().unwrap();

                    if poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::Value::LineString(_) => {
                    let line_string: LineString = (&geometry.value).try_into().unwrap();

                    if line_string.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(area_elements)
}

pub fn update_areas_tag(elements: &Vec<Element>, conn: &Connection) -> Result<Vec<Element>> {
    let mut res: Vec<Element> = vec![];

    let all_areas: Vec<Area> = Area::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();

    for element in elements {
        let element_areas = find_areas(element, &all_areas)?;
        let element_areas = areas_to_areas_tag(element_areas);

        let element = if element.tag("areas") != &element_areas {
            info!(
                element = element.id,
                old = serde_json::to_string(element.tag("areas"))?,
                new = serde_json::to_string(&element_areas)?,
                "Change detected, updating areas tag",
            );
            Element::set_tag(element.id, "areas", &element_areas, conn)?
        } else {
            info!(element = element.id, "No changes, skipping update");
            element.clone()
        };

        res.push(element);
    }

    Ok(res)
}

fn areas_to_areas_tag(areas: Vec<&Area>) -> Value {
    let element_areas: Vec<Value> = areas.iter().map(|it| {
        json!({"id": it.id, "url_alias": it.tags.get("url_alias").unwrap_or(&Value::Null).as_str().unwrap_or_default()})
    }).collect();
    Value::Array(element_areas)
}

pub fn find_areas<'a>(element: &Element, areas: &'a Vec<Area>) -> Result<Vec<&'a Area>> {
    let mut element_areas = vec![];

    for area in areas {
        if area.tags.get("url_alias") == Some(&Value::String("earth".into())) {
            continue;
        }

        let geometries = area.geo_json_geometries();

        for geometry in &geometries {
            match &geometry.value {
                geojson::Value::MultiPolygon(_) => {
                    let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                    if multi_poly.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                geojson::Value::Polygon(_) => {
                    let poly: Polygon = (&geometry.value).try_into().unwrap();

                    if poly.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                geojson::Value::LineString(_) => {
                    let line_string: LineString = (&geometry.value).try_into().unwrap();

                    if line_string.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(element_areas)
}

#[derive(Serialize, Deserialize)]
pub struct Issue {
    pub r#type: String,
    pub severity: i64,
    pub description: String,
}

pub fn generate_issues(elements: Vec<&Element>, conn: &Connection) -> Result<()> {
    for element in elements {
        let issues = crate::element::service::get_issues(&element);
        // No current issues, no saved issues, nothing to do here
        if issues.is_empty() && !element.tags.contains_key("issues") {
            return Ok(());
        }
        // No current issues found but an element has some old issues which need to be deleted
        if issues.is_empty() && element.tags.contains_key("issues") {
            Element::remove_tag(element.id, "issues", conn)?;
            return Ok(());
        }
        let issues = serde_json::to_value(&issues)?;
        // We should avoid toucing the elements if the issues didn't change
        if element.tag("issues") != &issues {
            Element::set_tag(element.id, "issues", &issues, conn)?;
        }
    }

    Ok(())
}

fn get_issues(element: &Element) -> Vec<Issue> {
    let mut res: Vec<Issue> = vec![];
    res.append(&mut crate::element::service::get_date_format_issues(
        element,
    ));
    res.append(&mut crate::element::service::get_misspelled_tag_issues(
        element,
    ));
    if let Some(issue) = crate::element::service::get_missing_icon_issue(element) {
        res.push(issue);
    };
    if let Some(issue) = crate::element::service::get_not_verified_issue(element) {
        res.push(issue);
    };
    if let Some(issue) = crate::element::service::get_out_of_date_issue(element) {
        res.push(issue);
    } else {
        if let Some(issue) = crate::element::service::get_soon_out_of_date_issue(element) {
            res.push(issue);
        };
    };
    res
}

fn get_date_format_issues(element: &Element) -> Vec<Issue> {
    let mut res: Vec<Issue> = vec![];
    let date_format = format_description!("[year]-[month]-[day]");
    let survey_date = element.overpass_data.tag("survey:date");
    if survey_date.len() > 0 && Date::parse(survey_date, &date_format).is_err() {
        res.push(Issue {
            r#type: "date_format".into(),
            severity: 600,
            description: "survey:date is not formatted properly".into(),
        });
    }
    let check_date = element.overpass_data.tag("check_date");
    if check_date.len() > 0 && Date::parse(check_date, &date_format).is_err() {
        res.push(Issue {
            r#type: "date_format".into(),
            severity: 600,
            description: "check_date is not formatted properly".into(),
        });
    }
    let check_date_currency_xbt = element.overpass_data.tag("check_date:currency:XBT");
    if check_date_currency_xbt.len() > 0
        && Date::parse(check_date_currency_xbt, &date_format).is_err()
    {
        res.push(Issue {
            r#type: "date_format".into(),
            severity: 600,
            description: "check_date:currency:XBT is not formatted properly".into(),
        });
    }
    res
}

fn get_misspelled_tag_issues(element: &Element) -> Vec<Issue> {
    let mut res: Vec<Issue> = vec![];
    let payment_lighting = element.overpass_data.tag("payment:lighting");
    if payment_lighting.len() > 0 {
        res.push(Issue {
            r#type: "misspelled_tag".into(),
            severity: 500,
            description: "Spelling issue: payment:lighting".into(),
        });
    }
    let payment_lightning_contacless = element.overpass_data.tag("payment:lightning_contacless");
    if payment_lightning_contacless.len() > 0 {
        res.push(Issue {
            r#type: "misspelled_tag".into(),
            severity: 500,
            description: "Spelling issue: payment:lightning_contacless".into(),
        });
    }
    let payment_lighting_contactless = element.overpass_data.tag("payment:lighting_contactless");
    if payment_lighting_contactless.len() > 0 {
        res.push(Issue {
            r#type: "misspelled_tag".into(),
            severity: 500,
            description: "Spelling issue: payment:lighting_contactless".into(),
        });
    }
    res
}

fn get_missing_icon_issue(element: &Element) -> Option<Issue> {
    if element.tag("icon:android").as_str().unwrap_or_default() == ""
        || element.tag("icon:android").as_str().unwrap_or_default() == "question_mark"
    {
        return Some(Issue {
            r#type: "missing_icon".into(),
            severity: 400,
            description: "Icon is missing".into(),
        });
    }

    None
}

fn get_not_verified_issue(element: &Element) -> Option<Issue> {
    if element.overpass_data.verification_date().is_none() {
        return Some(Issue {
            r#type: "not_verified".into(),
            severity: 300,
            description: "Not verified".into(),
        });
    }

    None
}

fn get_out_of_date_issue(element: &Element) -> Option<Issue> {
    if element.overpass_data.verification_date().is_some() && !element.overpass_data.up_to_date() {
        return Some(Issue {
            r#type: "out_of_date".into(),
            severity: 200,
            description: "Out of date".into(),
        });
    }

    None
}

fn get_soon_out_of_date_issue(element: &Element) -> Option<Issue> {
    if element.overpass_data.verification_date().is_some()
        && element
            .overpass_data
            .days_since_verified()
            .map(|it| it > 365 - 90 && it < 365)
            .is_some_and(|it| it)
    {
        return Some(Issue {
            r#type: "out_of_date_soon".into(),
            severity: 100,
            description: "Soon to be outdated".into(),
        });
    }

    None
}
