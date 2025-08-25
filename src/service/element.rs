use crate::db;
use crate::db::area::schema::Area;
use crate::db::element::schema::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
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
use url::Url;

pub async fn remove_areas_tag(pool: &Pool) -> Result<()> {
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, true, pool)
            .await?;
    info!(elements = elements.len(), "preparing to remove areas tag");
    for element in elements {
        if element.tags.contains_key("areas") {
            info!(element.id, "found legacy areas tag");
            db::element::queries::remove_tag(element.id, "areas", pool).await?;
        }
    }
    Ok(())
}

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
    let mut rough_matches = 0;

    for area in areas {
        if area.tags.get("url_alias") == Some(&Value::String("earth".into())) {
            continue;
        }

        let lat = element.lat();
        let lon = element.lon();

        if lon > area.bbox_west
            && lon < area.bbox_east
            && lat > area.bbox_south
            && lat < area.bbox_north
        {
            rough_matches += 1;
        } else {
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

    info!(element.id, element.name = element.name(), rough_matches);
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

pub async fn generate_issues(elements: Vec<&Element>, pool: &Pool) -> Result<GenerateIssuesResult> {
    let started_at = OffsetDateTime::now_utc();
    let mut affected_elements = 0;
    for element in elements {
        let issues = get_issues(element);
        let old_issues = db::element_issue::queries::select_by_element_id(element.id, pool).await?;
        for old_issue in &old_issues {
            let still_exists = issues.iter().find(|it| it.code() == old_issue.code);
            if old_issue.deleted_at.is_none() && still_exists.is_none() {
                db::element_issue::queries::set_deleted_at(
                    old_issue.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
            }
        }
        for issue in &issues {
            let old_issue = old_issues.iter().find(|it| it.code == issue.code());
            match old_issue {
                Some(old_issue) => {
                    if old_issue.deleted_at.is_some() {
                        db::element_issue::queries::set_deleted_at(old_issue.id, None, pool)
                            .await?;
                    }
                    if old_issue.severity != issue.severity {
                        db::element_issue::queries::set_severity(
                            old_issue.id,
                            issue.severity,
                            pool,
                        )
                        .await?;
                    }
                }
                None => {
                    db::element_issue::queries::insert(
                        element.id,
                        issue.code(),
                        issue.severity,
                        pool,
                    )
                    .await?;
                }
            }
        }
        // No current issues, no saved issues, nothing to do here
        if issues.is_empty() && !element.tags.contains_key("issues") {
            continue;
        }
        // No current issues found but an element has some old issues which need to be deleted
        if issues.is_empty() && element.tags.contains_key("issues") {
            db::element::queries::remove_tag(element.id, "issues", pool).await?;
            affected_elements += 1;
            continue;
        }
        let issues = serde_json::to_value(&issues)?;
        // We should avoid toucing the elements if the issues didn't change
        if element.tag("issues") != &issues {
            db::element::queries::set_tag(element.id, "issues", &issues, pool).await?;
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
    res.append(&mut get_date_format_issues(element));
    res.append(&mut get_misspelled_tag_issues(element));
    if let Some(issue) = get_missing_icon_issue(element) {
        res.push(issue);
    };
    if let Some(issue) = get_not_verified_issue(element) {
        res.push(issue);
    };
    if let Some(issue) = get_out_of_date_issue(element) {
        res.push(issue);
    } else if let Some(issue) = get_soon_out_of_date_issue(element) {
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

pub const TAGS: &[&str] = &[
    "osm_id",
    "osm_url",
    "lat",
    "lon",
    "name",
    "address",
    "icon",
    "phone",
    "website",
    "twitter",
    "facebook",
    "instagram",
    "line",
    "email",
    "opening_hours",
    "boosted_until",
    "required_app_url",
    "created_at",
    "updated_at",
    "deleted_at",
    "verified_at",
    "comments",
];

pub fn generate_tags(element: &Element, include_tags: &[&str]) -> Map<String, Value> {
    let mut res = Map::new();
    res.insert("id".into(), element.id.into());
    let include_tags: Vec<&str> = include_tags
        .iter()
        .copied()
        .filter(|it| TAGS.contains(it) || it.starts_with("osm:"))
        .collect();
    let empty_map = Map::new();
    let osm_tags = element.overpass_data.tags.as_ref().unwrap_or(&empty_map);
    for tag in &include_tags {
        match *tag {
            "icon" => match element.tags.get("icon:android") {
                Some(icon) => {
                    if icon.is_string() {
                        res.insert("icon".into(), icon.clone());
                    } else {
                        res.insert("icon".into(), "question_mark".into());
                    }
                }
                None => {
                    res.insert("icon".into(), "question_mark".into());
                }
            },
            "boosted_until" => {
                if element.tags.contains_key("boost:expires") {
                    res.insert(
                        "boosted_until".into(),
                        element.tags["boost:expires"].clone(),
                    );
                }
            }
            "name" => {
                let name = element.overpass_data.tag("name");
                if name.is_empty() {
                    res.insert("name".into(), "Unnamed".into());
                } else {
                    res.insert("name".into(), name.into());
                }
            }
            "opening_hours" => {
                if !element.overpass_data.tag("opening_hours").is_empty() {
                    res.insert(
                        "opening_hours".into(),
                        element.overpass_data.tag("opening_hours").into(),
                    );
                }
            }
            "required_app_url" => {
                let required_app_url = element
                    .overpass_data
                    .tag("payment:lightning:companion_app_url");
                if is_valid_url(required_app_url) {
                    res.insert("required_app_url".into(), required_app_url.into());
                }
            }
            "comments" => {
                if element.tags.contains_key("comments") {
                    res.insert("comments".into(), element.tags["comments"].clone());
                }
            }
            "phone" => {
                if !element.overpass_data.tag("phone").is_empty() {
                    res.insert("phone".into(), element.overpass_data.tag("phone").into());
                } else if !element.overpass_data.tag("contact:phone").is_empty() {
                    res.insert(
                        "phone".into(),
                        element.overpass_data.tag("contact:phone").into(),
                    );
                }
            }
            "website" => {
                if !element.overpass_data.tag("website").is_empty() {
                    let website = element.overpass_data.tag("website");
                    if is_valid_url(website) {
                        res.insert("website".into(), website.into());
                    }
                } else {
                    let website = element.overpass_data.tag("contact:website");
                    if is_valid_url(website) {
                        res.insert("website".into(), website.into());
                    }
                }
            }
            "twitter" => {
                let twitter = element.overpass_data.tag("contact:twitter");
                if is_valid_url(twitter) {
                    res.insert("twitter".into(), twitter.into());
                }
            }
            "facebook" => {
                let facebook = element.overpass_data.tag("contact:facebook");
                if is_valid_url(facebook) {
                    res.insert("facebook".into(), facebook.into());
                }
            }
            "instagram" => {
                let instagram = element.overpass_data.tag("contact:instagram");
                if is_valid_url(instagram) {
                    res.insert("instagram".into(), instagram.into());
                }
            }
            "line" => {
                let line = element.overpass_data.tag("contact:line");
                if is_valid_url(line) {
                    res.insert("line".into(), line.into());
                }
            }
            "email" => {
                if !element.overpass_data.tag("email").is_empty() {
                    res.insert("email".into(), element.overpass_data.tag("email").into());
                } else if !element.overpass_data.tag("contact:email").is_empty() {
                    res.insert(
                        "email".into(),
                        element.overpass_data.tag("contact:email").into(),
                    );
                }
            }
            "address" => {
                let mut addr = String::new();
                let housenumber = element.overpass_data.tag("addr:housenumber");
                if !housenumber.is_empty() {
                    addr.push_str(housenumber);
                    addr.push(' ');
                }
                let street = element.overpass_data.tag("addr:street");
                if !street.is_empty() {
                    addr.push_str(street);
                    addr.push(' ');
                }
                let city = element.overpass_data.tag("addr:city");
                if !city.is_empty() {
                    addr.push_str(city);
                    addr.push(' ');
                }
                let postcode = element.overpass_data.tag("addr:postcode");
                if !postcode.is_empty() {
                    addr.push_str(postcode);
                    addr.push(' ');
                }
                let addr = addr.trim();
                if !addr.is_empty() {
                    res.insert("address".into(), addr.into());
                }
            }
            "osm_id" => {
                res.insert("osm_id".into(), element.overpass_data.btcmap_id().into());
            }
            "osm_url" => {
                res.insert("osm_url".into(), element.osm_url().into());
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
            "deleted_at" => {
                if let Some(deleted_at) = element.deleted_at {
                    res.insert(
                        "deleted_at".into(),
                        Value::String(deleted_at.format(&Rfc3339).unwrap_or_default()),
                    );
                }
            }
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
        }
    }
    res
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(url) => url.scheme() == "http" || url.scheme() == "https",
        Err(_) => false,
    }
}
