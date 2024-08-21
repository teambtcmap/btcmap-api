use crate::{element::Element, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};
use time::{macros::format_description, Date, OffsetDateTime};

#[derive(Serialize)]
pub struct Report {
    pub created_at: OffsetDateTime,
    pub issues_count: i32,
    pub issues: Vec<ReportIssue>,
}

#[derive(Serialize)]
pub struct ReportIssue {
    pub r#type: String,
    pub severity: i64,
    pub description: String,
    pub osm_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Issue {
    pub r#type: String,
    pub severity: i64,
    pub description: String,
}

pub fn generate_issues(conn: &Connection) -> Result<()> {
    let elements: Vec<_> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    for element in elements {
        generate_element_issues(&element, conn)?;
        sleep(Duration::from_millis(1));
    }
    Ok(())
}

pub fn generate_element_issues(element: &Element, conn: &Connection) -> Result<()> {
    let issues = get_issues(&element);
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
    Ok(())
}

pub fn generate_report(conn: &Connection) -> Result<Report> {
    let elements: Vec<_> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    _generate_report(elements)
}

fn _generate_report(elements: Vec<Element>) -> Result<Report> {
    let mut issues: Vec<ReportIssue> = vec![];
    for element in &elements {
        if !element.tags.contains_key("issues") {
            continue;
        }
        let element_issues: Vec<Issue> = serde_json::from_value(element.tags["issues"].clone())?;
        let mut element_issues: Vec<ReportIssue> = element_issues
            .into_iter()
            .map(|it| ReportIssue {
                r#type: it.r#type,
                severity: it.severity,
                description: it.description,
                osm_url: format!(
                    "https://openstreetmap.org/{}/{}",
                    element.overpass_data.r#type, element.overpass_data.id,
                ),
            })
            .collect();
        issues.append(&mut element_issues);
    }
    issues.sort_by(|a, b| b.severity.cmp(&a.severity));
    Ok(Report {
        created_at: OffsetDateTime::now_utc(),
        issues_count: issues.len().try_into().unwrap(),
        issues,
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
    } else {
        if let Some(issue) = get_soon_out_of_date_issue(element) {
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
