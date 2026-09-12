use crate::{
    db,
    db::main::{area::schema::Area, place_submission::schema::PlaceSubmission},
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
    revocations_processed: i64,
}

const LOCATION_SUBMISSION_LABEL_ID: i64 = 901;
const LOCATION_REMOVAL_LABEL_ID: i64 = 904;

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

    let revoked_submissions =
        db::main::place_submission::queries::select_revoked_with_ticket_url(pool).await?;
    info!(
        len = revoked_submissions.len(),
        "fetched revoked submissions with gitea tickets",
    );

    let mut revocations_processed = 0;

    for submission in &revoked_submissions {
        revocations_processed += process_revoked_submission(submission, pool).await as i64;
    }

    Ok(Res {
        issues_pending: submissions.len() as i64 - issues_closed,
        issues_created,
        issues_closed,
        revocations_processed,
    })
}

#[derive(Debug, PartialEq)]
enum RevocationPlan {
    /// The ticket is still open, so no reviewer has actioned it yet: close it.
    CloseAndComment,
    /// The ticket is already closed, so the revocation needs to be surfaced
    /// again for a reviewer: reopen it.
    Reopen,
    /// The ticket already carries the removal label, so this revocation has
    /// been processed by an earlier run.
    AlreadyProcessed,
    /// Neither open nor closed, so the safest thing is to leave the ticket be.
    UnhandledState,
}

fn plan_revocation(issue_state: &str, has_removal_label: bool) -> RevocationPlan {
    if has_removal_label {
        return RevocationPlan::AlreadyProcessed;
    }
    match issue_state {
        "open" => RevocationPlan::CloseAndComment,
        "closed" => RevocationPlan::Reopen,
        _ => RevocationPlan::UnhandledState,
    }
}

async fn process_revoked_submission(submission: &PlaceSubmission, pool: &Pool) -> bool {
    let Some(ticket_url) = submission.ticket_url.clone() else {
        return false;
    };

    let issue = match service::gitea::get_issue(ticket_url.clone(), pool).await {
        Ok(Some(issue)) => issue,
        Ok(None) => {
            warn!(
                submission_id = submission.id,
                ticket_url = ticket_url,
                "revoked submission's gitea ticket not found"
            );
            return false;
        }
        Err(e) => {
            warn!(
                submission_id = submission.id,
                ticket_url = ticket_url,
                error = %e,
                "failed to fetch gitea ticket for revoked submission"
            );
            return false;
        }
    };

    let has_removal_label = issue
        .labels
        .iter()
        .any(|label| label.id == LOCATION_REMOVAL_LABEL_ID);

    let plan = plan_revocation(&issue.state, has_removal_label);

    if plan == RevocationPlan::UnhandledState {
        warn!(
            submission_id = submission.id,
            ticket_url = ticket_url,
            state = issue.state,
            "unexpected gitea ticket state for revoked submission"
        );
        return false;
    }

    if plan == RevocationPlan::AlreadyProcessed {
        return false;
    }

    // Relabel before changing the state: the removal label is what makes this
    // and every later run skip the submission, so a failure here leaves the
    // ticket untouched for the next run to retry.
    let removal_labels = build_removal_labels(&submission.origin, pool).await;
    if let Err(e) = service::gitea::set_issue_labels(&ticket_url, removal_labels, pool).await {
        warn!(
            submission_id = submission.id,
            ticket_url = ticket_url,
            error = %e,
            "failed to relabel gitea ticket for revoked submission"
        );
        return false;
    }

    match plan {
        RevocationPlan::CloseAndComment => {
            if let Err(e) = service::gitea::close_issue(&ticket_url, pool).await {
                warn!(
                    submission_id = submission.id,
                    ticket_url = ticket_url,
                    error = %e,
                    "failed to close gitea ticket for revoked submission"
                );
            }
            if let Err(e) = service::gitea::add_issue_comment(
                &ticket_url,
                "This location was revoked before being processed.",
                pool,
            )
            .await
            {
                warn!(
                    submission_id = submission.id,
                    ticket_url = ticket_url,
                    error = %e,
                    "failed to comment on gitea ticket for revoked submission"
                );
            }
            let message = format!(
                "Closed Gitea issue for revoked submission {} {}",
                submission.name, issue.html_url
            );
            let matrix_client = matrix::try_client(pool);
            service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
        }
        RevocationPlan::Reopen => {
            if let Err(e) = service::gitea::reopen_issue(&ticket_url, pool).await {
                warn!(
                    submission_id = submission.id,
                    ticket_url = ticket_url,
                    error = %e,
                    "failed to reopen gitea ticket for revoked submission"
                );
            }
            let message = format!(
                "Reopened Gitea issue for revoked submission {} {}",
                submission.name, issue.html_url
            );
            let matrix_client = matrix::try_client(pool);
            service::matrix::send_message(&matrix_client, ROOM_PLACE_IMPORT, &message);
        }
        RevocationPlan::AlreadyProcessed | RevocationPlan::UnhandledState => {}
    }

    true
}

async fn build_removal_labels(origin: &str, pool: &Pool) -> Vec<i64> {
    let mut labels = vec![LOCATION_REMOVAL_LABEL_ID];
    if let Ok(Some(import_origin)) =
        db::main::place_import_origin::queries::select_by_name(origin.to_string(), pool).await
    {
        if let Some(label_id) = import_origin.gitea_label_id {
            labels.push(label_id);
        }
    }
    labels
}

#[cfg(test)]
mod test {
    use super::{build_issue_title, plan_revocation, RevocationPlan};
    use crate::db::main::area::schema::Area;
    use crate::db::main::test::pool;
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

    #[test]
    fn open_ticket_is_closed() {
        assert_eq!(
            RevocationPlan::CloseAndComment,
            plan_revocation("open", false)
        );
    }

    #[test]
    fn closed_ticket_is_reopened() {
        assert_eq!(RevocationPlan::Reopen, plan_revocation("closed", false));
    }

    #[test]
    fn removal_label_marks_ticket_as_processed() {
        assert_eq!(
            RevocationPlan::AlreadyProcessed,
            plan_revocation("open", true)
        );
        assert_eq!(
            RevocationPlan::AlreadyProcessed,
            plan_revocation("closed", true)
        );
    }

    #[test]
    fn unknown_state_is_left_alone() {
        assert_eq!(
            RevocationPlan::UnhandledState,
            plan_revocation("broken", false)
        );
    }

    #[actix_web::test]
    async fn removal_labels_fall_back_to_default() {
        let pool = pool();
        let labels = super::build_removal_labels("unknown-origin", &pool).await;
        assert_eq!(labels, vec![super::LOCATION_REMOVAL_LABEL_ID]);
    }
}
