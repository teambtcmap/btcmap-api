use crate::{
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
            let title = submission.name.to_string();

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
