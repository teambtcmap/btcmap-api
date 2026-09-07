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

const LOCATION_SUBMISSION_LABEL_ID: i64 = 901;

fn build_issue_title(areas: &[Area], name: &str) -> String {
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
        name.to_string()
    } else {
        format!("{} {}", prefix, name)
    }
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let submissions =
        db::main::place_submission::queries::select_open_and_not_revoked(pool).await?;
    info!(
        len = submissions.len(),
        "fetched open and non-revoked submissions",
    );

    let mut issues_created = 0;
    let mut issues_closed = 0;

    for submission in &submissions {
        let Some(import_origin) =
            db::main::place_import_origin::queries::select_by_name(submission.origin.clone(), pool)
                .await?
        else {
            warn!(submission.origin, "unknown origin");
            continue;
        };

        if !import_origin.gitea_sync_enabled {
            warn!(submission.origin, "disabled origin");
            continue;
        }

        if submission.ticket_url.is_none() {
            let areas =
                service::area::find_areas_by_lat_lon(submission.lat, submission.lon, pool).await?;
            let title = build_issue_title(&areas, &submission.name);

            let body = format!(
                r#"
                Id: {id}
                Origin: {origin}
                Name: {name}
                Category: {category}

                Extra fields:

                {extra_fields}

                OpenStreetMap viewer link: https://www.openstreetmap.org/#map=21/{lat}/{lon}

                OpenStreetMap editor link: https://www.openstreetmap.org/edit#map=21/{lat}/{lon}

                To verify this imported place:

                1. Check if the place already exists in OSM.
                2. If it exists: Confirm it has a currency:XBT tag, then close this ticket.
                3. If it does not exist: Contact the merchant or verify its existence using at least one other source.

                Check this page for more instructions if you're just starting as an OSM contributor:

                https://gitea.btcmap.org/teambtcmap/btcmap-general/wiki/Tagging-Merchants
            "#,
                id = submission.id,
                origin = submission.origin,
                name = submission.name,
                category = submission.category,
                extra_fields = serde_json::to_string_pretty(&submission.extra_fields)?,
                lat = submission.lat,
                lon = submission.lon,
            );
            let body = body
                .lines()
                .map(|line| line.trim())
                .collect::<Vec<&str>>()
                .join("\n");
            let mut label_ids = vec![LOCATION_SUBMISSION_LABEL_ID];
            if let Some(gitea_label_id) = import_origin.gitea_label_id {
                label_ids.push(gitea_label_id);
            }
            let issue = service::gitea::create_issue(title, body, label_ids, pool).await?;
            db::main::place_submission::queries::set_ticket_url(
                submission.id,
                issue.url.clone(),
                pool,
            )
            .await?;
            issues_created += 1;
            let message = format!(
                "Created Gitea issue for {} {}",
                submission.name, issue.html_url
            );
            let matrix_client = matrix::try_client(pool);
            service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
        } else {
            let issue =
                service::gitea::get_issue(submission.ticket_url.clone().unwrap(), pool).await?;

            let Some(issue) = issue else {
                continue;
            };

            if issue.state == "closed" {
                db::main::place_submission::queries::set_closed_at(
                    submission.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
                issues_closed += 1;
                let message = format!(
                    "Closed Gitea issue and marked submission as closed for {} {}",
                    submission.name, issue.html_url
                );
                let matrix_client = matrix::try_client(pool);
                service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
            }
        }
    }

    Ok(Res {
        issues_pending: submissions.len() as i64 - issues_closed,
        issues_created,
        issues_closed,
    })
}

#[cfg(test)]
mod test {
    use super::build_issue_title;
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
            build_issue_title(&areas, "Some Cafe"),
            "[TH][Phuket Bitcoin Community] Some Cafe",
        );
    }

    #[test]
    fn title_with_country_only_uppercases_alias() {
        let areas = vec![area("country", "Thailand", "th")];
        assert_eq!(build_issue_title(&areas, "Some Cafe"), "[TH] Some Cafe");
    }

    #[test]
    fn title_with_community_only_uses_name_as_is() {
        let areas = vec![area("community", "Phuket Bitcoin Community", "phuket")];
        assert_eq!(
            build_issue_title(&areas, "Some Cafe"),
            "[Phuket Bitcoin Community] Some Cafe",
        );
    }

    #[test]
    fn title_without_areas_falls_back_to_name() {
        assert_eq!(build_issue_title(&[], "Some Cafe"), "Some Cafe");
    }

    #[test]
    fn title_ignores_unrelated_area_types() {
        let areas = vec![area("planet", "Earth", "earth")];
        assert_eq!(build_issue_title(&areas, "Some Cafe"), "Some Cafe");
    }
}
