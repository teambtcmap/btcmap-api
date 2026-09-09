use crate::{
    db::main::area::schema::Area,
    db::{self},
    service::matrix::ROOM_PLACE_IMPORT,
    service::{self, matrix},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;
use tracing::{info, warn};

#[derive(Serialize)]
pub struct Res {
    issues_pending: i64,
    issues_created: i64,
    issues_closed: i64,
}

const PLACE_REPORT_LABEL_ID: i64 = 903;
const PLACE_REPORT_REMOVAL_LABEL_ID: i64 = 904;

fn needs_removal_label(report_type: &str) -> bool {
    matches!(report_type, "refused_sats" | "out_of_business")
}

fn build_issue_title(areas: &[Area], name: &str, report_type: &str) -> String {
    let country = areas
        .iter()
        .find(|area| area.tags.get("type").and_then(|v| v.as_str()) == Some("country"));
    let community = areas
        .iter()
        .find(|area| area.tags.get("type").and_then(|v| v.as_str()) == Some("community"));

    let mut prefix = String::new();
    if let Some(country) = country {
        prefix.push_str(&format!("[{}]", country.alias().to_uppercase()));
    }
    if let Some(community) = community {
        prefix.push_str(&format!("[{}]", community.name()));
    }

    if prefix.is_empty() {
        format!("[{}] {}", report_type, name)
    } else {
        format!("{} [{}] {}", prefix, report_type, name)
    }
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let reports = db::main::place_report::queries::select_open_and_not_deleted(pool).await?;
    info!(
        len = reports.len(),
        "fetched open and non-deleted place reports"
    );

    let mut issues_created = 0;
    let mut issues_closed = 0;

    for report in &reports {
        let Some(import_origin) =
            db::main::place_import_origin::queries::select_by_id(report.origin_id, pool).await?
        else {
            warn!(report.origin_id, "unknown origin");
            continue;
        };

        if !import_origin.gitea_sync_enabled {
            warn!(report.origin_id, "disabled origin");
            continue;
        }

        let element = match db::main::element::queries::select_by_id(report.place_id, pool).await {
            Ok(element) => element,
            Err(crate::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => {
                warn!(report.place_id, "missing element");
                continue;
            }
            Err(e) => return Err(e),
        };

        if report.ticket_url.is_none() {
            let areas =
                service::area::find_areas_by_lat_lon(element.lat(), element.lon(), pool).await?;
            let title = build_issue_title(&areas, &element.name(None), &report.r#type);

            let body = format!(
                r#"
                Id: {id}
                Origin: {origin}
                Place id: {place_id}
                Type: {type}

                Extra fields:

                {extra_fields}

                OpenStreetMap viewer link: https://www.openstreetmap.org/#map=21/{lat}/{lon}

                OpenStreetMap editor link: https://www.openstreetmap.org/edit#map=21/{lat}/{lon}

                To resolve this report:

                1. Look up the place in OSM and on BTC Map using the place id and links above.
                2. Decide whether the report is actionable, then update the place or close the ticket.
            "#,
                id = report.id,
                origin = import_origin.name,
                place_id = report.place_id,
                type = report.r#type,
                extra_fields = serde_json::to_string_pretty(&report.extra_fields)?,
                lat = element.lat(),
                lon = element.lon(),
            );
            let body = body
                .lines()
                .map(|line| line.trim())
                .collect::<Vec<&str>>()
                .join("\n");
            let mut label_ids = vec![PLACE_REPORT_LABEL_ID];
            if needs_removal_label(&report.r#type) {
                label_ids.push(PLACE_REPORT_REMOVAL_LABEL_ID);
            }
            let issue = service::gitea::create_issue(title, body, label_ids, pool).await?;
            db::main::place_report::queries::set_ticket_url(report.id, issue.url.clone(), pool)
                .await?;
            issues_created += 1;
            let message = format!(
                "Created Gitea issue for place report {} {}",
                report.id, issue.html_url
            );
            let matrix_client = matrix::try_client(pool);
            service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
        } else {
            let issue = service::gitea::get_issue(report.ticket_url.clone().unwrap(), pool).await?;

            let Some(issue) = issue else {
                continue;
            };

            if issue.state == "closed" {
                db::main::place_report::queries::set_closed_at(
                    report.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
                issues_closed += 1;
                let message = format!(
                    "Closed Gitea issue and marked report as closed for {} {}",
                    report.id, issue.html_url
                );
                let matrix_client = matrix::try_client(pool);
                service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
            }
        }
    }

    Ok(Res {
        issues_pending: reports.len() as i64 - issues_closed,
        issues_created,
        issues_closed,
    })
}

#[cfg(test)]
mod test {
    use super::{build_issue_title, needs_removal_label};
    use crate::db::main::area::schema::Area;
    use serde_json::{Map, Value};
    use time::OffsetDateTime;

    fn area(area_type: &str, name: &str, alias: &str) -> Area {
        let mut tags = Map::new();
        tags.insert("type".into(), Value::String(area_type.into()));
        tags.insert("name".into(), Value::String(name.into()));
        tags.insert("url_alias".into(), Value::String(alias.into()));
        Area {
            id: 0,
            alias: alias.into(),
            bbox_west: 0.0,
            bbox_south: 0.0,
            bbox_east: 0.0,
            bbox_north: 0.0,
            tags,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            deleted_at: None,
        }
    }

    #[test]
    fn title_with_country_and_community() {
        let areas = vec![
            area("country", "Thailand", "th"),
            area("community", "Phuket Bitcoin Community", "phuket"),
        ];
        assert_eq!(
            build_issue_title(&areas, "Some Cafe", "verified"),
            "[TH][Phuket Bitcoin Community] [verified] Some Cafe",
        );
    }

    #[test]
    fn title_with_country_only_uppercases_alias() {
        let areas = vec![area("country", "Thailand", "th")];
        assert_eq!(
            build_issue_title(&areas, "Some Cafe", "verified"),
            "[TH] [verified] Some Cafe",
        );
    }

    #[test]
    fn title_with_community_only_uses_name_as_is() {
        let areas = vec![area("community", "Phuket Bitcoin Community", "phuket")];
        assert_eq!(
            build_issue_title(&areas, "Some Cafe", "verified"),
            "[Phuket Bitcoin Community] [verified] Some Cafe",
        );
    }

    #[test]
    fn title_without_areas_falls_back_to_name() {
        assert_eq!(
            build_issue_title(&[], "Some Cafe", "verified"),
            "[verified] Some Cafe",
        );
    }

    #[test]
    fn title_ignores_unrelated_area_types() {
        let areas = vec![area("planet", "Earth", "earth")];
        assert_eq!(
            build_issue_title(&areas, "Some Cafe", "verified"),
            "[verified] Some Cafe",
        );
    }

    #[test]
    fn removal_label_applies_to_refused_sats_and_out_of_business() {
        assert!(needs_removal_label("refused_sats"));
        assert!(needs_removal_label("out_of_business"));
        assert!(!needs_removal_label("verified"));
        assert!(!needs_removal_label("verification"));
        assert!(!needs_removal_label(""));
    }
}
