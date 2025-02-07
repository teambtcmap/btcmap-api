use super::Element;
use crate::area::Area;
use crate::Result;
use deadpool_sqlite::Pool;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;
use tracing::info;

pub fn filter_by_area(all_elements: &Vec<Element>, area: &Area) -> Result<Vec<Element>> {
    let geometries = area.geo_json_geometries()?;
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

pub fn find_areas<'a>(element: &Element, areas: &'a Vec<Area>) -> Result<Vec<&'a Area>> {
    let mut element_areas = vec![];
    info!("iterating all areas");
    for area in areas {
        //info!(area_id = area.id, area_name = area.name(), "iterating area");
        if area.tags.get("url_alias") == Some(&Value::String("earth".into())) {
            continue;
        }

        let geometries = area.geo_json_geometries()?;

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

pub struct GenerateIssuesResult {
    pub started_at: OffsetDateTime,
    pub finished_at: OffsetDateTime,
    pub time_s: f64,
    pub affected_elements: i64,
}

pub async fn generate_issues_async(
    elements: Vec<Element>,
    pool: &Pool,
) -> Result<GenerateIssuesResult> {
    pool.get()
        .await?
        .interact(move |conn| generate_issues(elements.iter().collect(), conn))
        .await?
}

pub fn generate_issues(elements: Vec<&Element>, conn: &Connection) -> Result<GenerateIssuesResult> {
    let started_at = OffsetDateTime::now_utc();
    let mut affected_elements = 0;
    for element in elements {
        let issues = crate::element::service::get_issues(element);
        // No current issues, no saved issues, nothing to do here
        if issues.is_empty() && !element.tags.contains_key("issues") {
            continue;
        }
        // No current issues found but an element has some old issues which need to be deleted
        if issues.is_empty() && element.tags.contains_key("issues") {
            Element::remove_tag(element.id, "issues", conn)?;
            affected_elements += 1;
            continue;
        }
        let issues = serde_json::to_value(&issues)?;
        // We should avoid toucing the elements if the issues didn't change
        if element.tag("issues") != &issues {
            Element::set_tag(element.id, "issues", &issues, conn)?;
            affected_elements += 1;
        }
    }
    let finished_at = OffsetDateTime::now_utc();
    Ok(GenerateIssuesResult {
        started_at,
        finished_at,
        time_s: (finished_at - started_at).as_seconds_f64(),
        affected_elements,
    })
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
    } else if let Some(issue) = crate::element::service::get_soon_out_of_date_issue(element) {
        res.push(issue);
    };
    res
}

fn get_date_format_issues(element: &Element) -> Vec<Issue> {
    let mut res: Vec<Issue> = vec![];
    let date_format = format_description!("[year]-[month]-[day]");
    let survey_date = element.overpass_data.tag("survey:date");
    if !survey_date.is_empty() && Date::parse(survey_date, &date_format).is_err() {
        res.push(Issue {
            r#type: "date_format".into(),
            severity: 600,
            description: "survey:date is not formatted properly".into(),
        });
    }
    let check_date = element.overpass_data.tag("check_date");
    if !check_date.is_empty() && Date::parse(check_date, &date_format).is_err() {
        res.push(Issue {
            r#type: "date_format".into(),
            severity: 600,
            description: "check_date is not formatted properly".into(),
        });
    }
    let check_date_currency_xbt = element.overpass_data.tag("check_date:currency:XBT");
    if !check_date_currency_xbt.is_empty()
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
    if !payment_lighting.is_empty() {
        res.push(Issue {
            r#type: "misspelled_tag".into(),
            severity: 500,
            description: "Spelling issue: payment:lighting".into(),
        });
    }
    let payment_lightning_contacless = element.overpass_data.tag("payment:lightning_contacless");
    if !payment_lightning_contacless.is_empty() {
        res.push(Issue {
            r#type: "misspelled_tag".into(),
            severity: 500,
            description: "Spelling issue: payment:lightning_contacless".into(),
        });
    }
    let payment_lighting_contactless = element.overpass_data.tag("payment:lighting_contactless");
    if !payment_lighting_contactless.is_empty() {
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

pub const TAGS: &'static [&str] = &[
    "name",
    "phone",
    "website",
    "check_date",
    "survey:date",
    "check_date:currency:XBT",
    "addr:street",
    "addr:housenumber",
    "contact:website",
    "opening_hours",
    "contact:phone",
    "contact:email",
    "contact:twitter",
    "contact:instagram",
    "contact:facebook",
    "contact:line",
    "btcmap:icon",
    "btcmap:boost:expires",
    "btcmap:osm:type",
    "btcmap:osm:id",
    "btcmap:osm:url",
    "btcmap:created_at",
    "btcmap:updated_at",
    "btcmap:deleted_at",
];

pub fn generate_tags(element: &Element, include_tags: &[&str]) -> Map<String, Value> {
    let mut res = Map::new();
    let include_tags: Vec<&str> = include_tags
        .to_vec()
        .into_iter()
        .filter(|it| TAGS.contains(it))
        .collect();
    if let Some(osm_tags) = &element.overpass_data.tags {
        for tag in &include_tags {
            if tag.starts_with("btcmap:") || tag.starts_with("osm:") {
                continue;
            }
            if osm_tags.contains_key(*tag) {
                res.insert(tag.to_string(), osm_tags[*tag].clone());
            }
        }
    }
    if element.tags.contains_key("icon:android") && include_tags.contains(&"btcmap:icon") {
        res.insert("btcmap:icon".into(), element.tags["icon:android"].clone());
    }
    if element.tags.contains_key("boost:expires") && include_tags.contains(&"btcmap:boost:expires")
    {
        res.insert(
            "btcmap:boost:expires".into(),
            element.tags["boost:expires"].clone(),
        );
    }
    if include_tags.contains(&"btcmap:osm:type") {
        res.insert(
            "btcmap:osm:type".into(),
            Value::String(element.overpass_data.r#type.clone()),
        );
    }
    if include_tags.contains(&"btcmap:osm:id") {
        res.insert(
            "btcmap:osm:id".into(),
            Value::Number(element.overpass_data.id.into()),
        );
    }
    if include_tags.contains(&"btcmap:osm:url") {
        res.insert("btcmap:osm:url".into(), Value::String(element.osm_url()));
    }
    if include_tags.contains(&"btcmap:created_at") {
        res.insert(
            "btcmap:created_at".into(),
            Value::String(element.created_at.format(&Rfc3339).unwrap_or_default()),
        );
    }
    if include_tags.contains(&"btcmap:updated_at") {
        res.insert(
            "btcmap:updated_at".into(),
            Value::String(element.updated_at.format(&Rfc3339).unwrap_or_default()),
        );
    }
    if include_tags.contains(&"btcmap:deleted_at") {
        match element.deleted_at {
            Some(deleted_at) => {
                res.insert(
                    "btcmap:deleted_at".into(),
                    Value::String(deleted_at.format(&Rfc3339).unwrap_or_default()),
                );
            }
            None => {}
        }
    }
    res
}
