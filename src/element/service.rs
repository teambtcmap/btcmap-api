use super::Element;
use crate::area::Area;
use crate::element_issue::model::ElementIssue;
use crate::Result;
use deadpool_sqlite::Pool;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
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

impl Issue {
    fn code(&self) -> String {
        match self.description.as_str() {
            "survey:date is not formatted properly" => "invalid_tag_value:survey:date",
            "check_date is not formatted properly" => "invalid_tag_value:check_date",
            "check_date:currency:XBT is not formatted properly" => {
                "invalid_tag_value:check_date:currency:XBT"
            }
            "Spelling issue: payment:lighting" => "misspelled_tag_name:payment:lighting",
            "Spelling issue: payment:lightning_contacless" => {
                "misspelled_tag_name:lightning_contacless"
            }
            "Spelling issue: payment:lighting_contactless" => {
                "misspelled_tag_name:lighting_contactless"
            }
            "Icon is missing" => "missing_icon",
            "Not verified" => "not_verified",
            "Out of date" => "outdated",
            "Soon to be outdated" => "outdated_soon",
            _ => "unknown",
        }
        .to_string()
    }
}

pub fn generate_issues(elements: Vec<&Element>, conn: &Connection) -> Result<GenerateIssuesResult> {
    let started_at = OffsetDateTime::now_utc();
    let mut affected_elements = 0;
    for element in elements {
        let issues = crate::element::service::get_issues(element);
        let old_issues = ElementIssue::select_by_element_id(element.id, conn)?;
        for old_issue in &old_issues {
            let still_exists = issues.iter().find(|it| it.code() == old_issue.code);
            if old_issue.deleted_at.is_none() && still_exists.is_none() {
                ElementIssue::set_deleted_at(old_issue.id, Some(OffsetDateTime::now_utc()), conn)?;
            }
        }
        for issue in &issues {
            let old_issue = old_issues.iter().find(|it| it.code == issue.code());
            match old_issue {
                Some(old_issue) => {
                    match old_issue.deleted_at {
                        Some(_) => {
                            ElementIssue::set_deleted_at(old_issue.id, None, conn)?;
                        }
                        None => {}
                    }
                    if old_issue.severity != issue.severity {
                        ElementIssue::set_severity(old_issue.id, issue.severity, conn)?;
                    }
                }
                None => {
                    ElementIssue::insert(element.id, issue.code(), issue.severity, conn)?;
                }
            }
        }
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
    "osm:type",
    "osm:id",
    "osm:url",
    "lat",
    "lon",
    "name",
    "address",
    "icon",
    "phone",
    "website",
    "twitter",
    "facebook",
    "instagramm",
    "line",
    "email",
    "boost:expires",
    "needs_app",
    "created_at",
    "updated_at",
    "deleted_at",
    "verified_at",
];

pub fn generate_tags(element: &Element, include_tags: &[&str]) -> Map<String, Value> {
    let mut res = Map::new();
    let include_tags: Vec<&str> = include_tags
        .to_vec()
        .into_iter()
        .filter(|it| TAGS.contains(it) || it.starts_with("osm:"))
        .collect();
    if let Some(osm_tags) = &element.overpass_data.tags {
        for tag in &include_tags {
            match *tag {
                "icon" => {
                    if element.tags.contains_key("icon:android") {
                        res.insert("icon".into(), element.tags["icon:android"].clone());
                    }
                }
                "boost:expires" => {
                    if element.tags.contains_key("boost:expires") {
                        res.insert(
                            "boost:expires".into(),
                            element.tags["boost:expires"].clone(),
                        );
                    }
                }
                "name" => {
                    if !element.overpass_data.tag("name").is_empty() {
                        res.insert("name".into(), element.overpass_data.tag("name").into());
                    }
                }
                "needs_app" => {
                    if !element.overpass_data.tag("payment:lightning:companion_app_url").is_empty() {
                        res.insert("needs_app".into(), element.overpass_data.tag("payment:lightning:companion_app_url").into());
                    }
                }
                "phone" => {
                    if !element.overpass_data.tag("phone").is_empty() {
                        res.insert("phone".into(), element.overpass_data.tag("phone").into());
                    } else {
                        if !element.overpass_data.tag("contact:phone").is_empty() {
                            res.insert("phone".into(), element.overpass_data.tag("contact:phone").into());
                        }
                    }
                }
                "website" => {
                    if !element.overpass_data.tag("website").is_empty() {
                        res.insert("website".into(), element.overpass_data.tag("website").into());
                    } else {
                        if !element.overpass_data.tag("contact:website").is_empty() {
                            res.insert("website".into(), element.overpass_data.tag("contact:website").into());
                        }
                    }
                }
                "twitter" => {
                    if !element.overpass_data.tag("contact:twitter").is_empty() {
                        res.insert("twitter".into(), element.overpass_data.tag("contact:twitter").into());
                    }
                }
                "facebook" => {
                    if !element.overpass_data.tag("contact:facebook").is_empty() {
                        res.insert("facebook".into(), element.overpass_data.tag("contact:facebook").into());
                    }
                }
                "instagram" => {
                    if !element.overpass_data.tag("contact:instagram").is_empty() {
                        res.insert("instagram".into(), element.overpass_data.tag("contact:instagram").into());
                    }
                }
                "line" => {
                    if !element.overpass_data.tag("contact:line").is_empty() {
                        res.insert("line".into(), element.overpass_data.tag("contact:line").into());
                    }
                }
                "email" => {
                    if !element.overpass_data.tag("email").is_empty() {
                        res.insert("email".into(), element.overpass_data.tag("email").into());
                    } else {
                        if !element.overpass_data.tag("contact:email").is_empty() {
                            res.insert("email".into(), element.overpass_data.tag("contact:email").into());
                        }
                    }
                }
                "address" => {
                    let mut addr = String::new();
                    let housenumber = element.overpass_data.tag("addr:housenumber");
                    if !housenumber.is_empty() {
                        addr.push_str(housenumber);
                        addr.push_str(" ");
                    }
                    let street = element.overpass_data.tag("addr:street");
                    if !street.is_empty() {
                        addr.push_str(street);
                        addr.push_str(" ");
                    }
                    let city = element.overpass_data.tag("addr:city");
                    if !city.is_empty() {
                        addr.push_str(city);
                        addr.push_str(" ");
                    }
                    let postcode = element.overpass_data.tag("addr:postcode");
                    if !postcode.is_empty() {
                        addr.push_str(postcode);
                        addr.push_str(" ");
                    }
                    let addr = addr.trim();
                    if !addr.is_empty() {
                        res.insert("address".into(), addr.into());
                    }
                }
                "osm:type" => {
                    res.insert(
                        "osm:type".into(),
                        Value::String(element.overpass_data.r#type.clone()),
                    );
                }
                "osm:id" => {
                    res.insert(
                        "osm:id".into(),
                        Value::Number(element.overpass_data.id.into()),
                    );
                }
                "osm:url" => {
                    res.insert("osm:url".into(), Value::String(element.osm_url()));
                }
                "created_at" => {
                    res.insert(
                        "created_at".into(),
                        Value::String(element.created_at.format(&Rfc3339).unwrap_or_default()),
                    );
                }
                "updated_at" => {
                    res.insert(
                        "updated_at".into(),
                        Value::String(element.updated_at.format(&Rfc3339).unwrap_or_default()),
                    );
                }
                "deleted_at" => match element.deleted_at {
                    Some(deleted_at) => {
                        res.insert(
                            "deleted_at".into(),
                            Value::String(deleted_at.format(&Rfc3339).unwrap_or_default()),
                        );
                    }
                    None => {}
                },
                "lat" => {
                    res.insert("lat".into(), json! {element.lat()});
                }
                "lon" => {
                    res.insert("lon".into(), json! {element.lon()});
                }
                "verified_at" => {
                    if let Some(date) = element.overpass_data.verification_date() {
                        res.insert("verified_at".into(), json! {date.date().to_string()});
                    }
                }
                unrecognized_tag => {
                    if unrecognized_tag.starts_with("osm:") {
                        let osm_tag = unrecognized_tag.trim_start_matches("osm:");
                        if osm_tags.contains_key(osm_tag) {
                            res.insert(tag.to_string(), osm_tags[osm_tag].clone());
                        }
                    }
                }
            };
        }
    }
    res
}
