use crate::{
    db::main::area::schema::Area,
    db::main::place_submission::schema::{PlaceSubmission, RevocationAction},
    db::{self},
    service::issue_body::{additional_fields, extra_value, field, humanize_list, single_line},
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

const TAGGING_GUIDELINES_URL: &str =
    "https://gitea.btcmap.org/teambtcmap/btcmap-general/wiki/Tagging-Merchants";

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

/// `extra_fields` is free-form, so every human-friendly line accepts the key
/// names our import origins actually send, in priority order.
const ENGLISH_NAME_KEYS: &[&str] = &["name:en", "name_en"];
const ADDRESS_KEYS: &[&str] = &["address"];
const PHONE_KEYS: &[&str] = &["phone"];
const OPENING_HOURS_KEYS: &[&str] = &["opening_hours"];
const WEBSITE_KEYS: &[&str] = &["website"];
const PAYMENT_METHODS_KEYS: &[&str] = &["payment_methods"];
const CONTACT_KEYS: &[&str] = &["contact", "email"];
const NOTES_KEYS: &[&str] = &["notes", "comment", "description"];
const DESCRIPTION_KEYS: &[&str] = &["description"];

/// Every `extra_fields` key consumed by the human-friendly section or the OSM
/// tag block. Whatever is left over is listed verbatim under "Additional
/// fields" so a source can keep sending extra data without it being silently
/// dropped. `osm:<tag>` passthrough keys are consumed dynamically and don't
/// appear here.
const CONSUMED_KEY_GROUPS: &[&[&str]] = &[
    ENGLISH_NAME_KEYS,
    ADDRESS_KEYS,
    PHONE_KEYS,
    OPENING_HOURS_KEYS,
    WEBSITE_KEYS,
    PAYMENT_METHODS_KEYS,
    CONTACT_KEYS,
    NOTES_KEYS,
    DESCRIPTION_KEYS,
];

/// BTC Map's own OSM tag flavour for the payment methods a source may report.
const PAYMENT_ONCHAIN_TAG: &str = "payment:onchain";
const PAYMENT_LIGHTNING_TAG: &str = "payment:lightning";
const PAYMENT_LIGHTNING_CONTACTLESS_TAG: &str = "payment:lightning_contactless";

/// The non-technical half of the issue: what the reviewer needs to know about
/// the place, with empty lines omitted and field groups separated by a blank
/// line. `extra_fields` the mapping doesn't know about still show up, under
/// "Additional fields".
fn build_human_section(submission: &PlaceSubmission) -> String {
    let extra = |keys: &[&str]| extra_value(&submission.extra_fields, keys);

    let groups: Vec<Vec<Option<String>>> = vec![
        vec![
            Some(field("Name (local)", &submission.name)),
            extra(ENGLISH_NAME_KEYS).map(|value| field("Name (English)", &value)),
            Some(field("Category", &submission.category)),
        ],
        vec![
            extra(ADDRESS_KEYS).map(|value| field("Address", &value)),
            extra(PHONE_KEYS).map(|value| field("Phone", &value)),
            extra(OPENING_HOURS_KEYS).map(|value| field("Opening hours", &value)),
        ],
        vec![
            extra(WEBSITE_KEYS).map(|value| field("Website", &value)),
            extra(PAYMENT_METHODS_KEYS)
                .map(|value| field("Payment methods", &humanize_list(&value))),
            extra(CONTACT_KEYS).map(|value| field("Contact", &value)),
        ],
        vec![extra(NOTES_KEYS).map(|value| field("Notes", &value))],
    ];

    let mut lines: Vec<String> = Vec::new();
    for group in groups {
        let group = group.into_iter().flatten().collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(group);
    }

    let mut consumed: Vec<&str> = CONSUMED_KEY_GROUPS.concat();
    consumed.extend(
        osm_tag_passthrough(submission)
            .into_iter()
            .map(|(key, _)| key),
    );
    let additional = additional_fields(&submission.extra_fields, &consumed);
    if !additional.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("Additional fields:".to_string());
        lines.extend(additional);
    }

    lines.join("\n")
}

/// Maps the source's `payment_methods` tokens onto the tags BTC Map uses.
/// Unrecognised tokens are dropped rather than guessed at.
fn payment_tags(payment_methods: Option<&str>) -> Vec<(&'static str, String)> {
    let Some(payment_methods) = payment_methods else {
        return Vec::new();
    };

    let mut keys: Vec<&'static str> = Vec::new();
    for token in payment_methods.split([',', ';']).map(|it| it.trim()) {
        let key = match token.to_lowercase().as_str() {
            "onchain" | "on-chain" => PAYMENT_ONCHAIN_TAG,
            "lightning" => PAYMENT_LIGHTNING_TAG,
            "nfc" => PAYMENT_LIGHTNING_CONTACTLESS_TAG,
            _ => continue,
        };
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.into_iter()
        .map(|key| (key, "yes".to_string()))
        .collect()
}

/// `"name@example.com"` gets the `email` tag; anything else a source puts in
/// `contact` is passed through under `contact`, where OSM accepts free text.
fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && !domain.is_empty() && !domain.contains('@') && domain.contains('.')
}

/// `(extra_fields key, value)` for entries using the documented `osm:<tag>`
/// passthrough (e.g. `osm:addr:city`) and holding a non-empty value. Sorted by
/// key so the issue body is stable regardless of JSON ordering.
fn osm_tag_passthrough(submission: &PlaceSubmission) -> Vec<(&str, String)> {
    let mut res: Vec<(&str, String)> = submission
        .extra_fields
        .iter()
        .filter_map(|(key, value)| {
            let tag = key.strip_prefix("osm:")?;
            if tag.is_empty() {
                return None;
            }
            Some((key.as_str(), single_line(value)?))
        })
        .collect();
    res.sort();
    res
}

/// Appends a tag unless its key is already present, so a field mapped from the
/// canonical key wins over an `osm:<tag>` passthrough and no key appears twice.
fn push_tag(tags: &mut Vec<(String, String)>, key: &str, value: String) {
    if !tags.iter().any(|(existing, _)| existing == key) {
        tags.push((key.to_string(), value));
    }
}

/// OSM tags a tagger can paste straight into their editor's tag paste, built
/// from the values the source actually sent: the canonical `extra_fields` keys
/// plus any `osm:<tag>` passthrough.
fn build_osm_tags(submission: &PlaceSubmission) -> String {
    let extra = |keys: &[&str]| extra_value(&submission.extra_fields, keys);

    let mut tags: Vec<(String, String)> = vec![("name".to_string(), submission.name.clone())];
    if let Some(value) = extra(ENGLISH_NAME_KEYS) {
        push_tag(&mut tags, "name:en", value);
    }
    if let Some(value) = extra(ADDRESS_KEYS) {
        push_tag(&mut tags, "addr:full", value);
    }
    if let Some(value) = extra(PHONE_KEYS) {
        push_tag(&mut tags, "phone", value);
    }
    if let Some(value) = extra(OPENING_HOURS_KEYS) {
        push_tag(&mut tags, "opening_hours", value);
    }
    if let Some(value) = extra(WEBSITE_KEYS) {
        push_tag(&mut tags, "website", value);
    }
    if let Some(value) = extra(CONTACT_KEYS) {
        let key = if is_email(&value) { "email" } else { "contact" };
        push_tag(&mut tags, key, value);
    }
    if let Some(value) = extra(DESCRIPTION_KEYS) {
        push_tag(&mut tags, "description", value);
    }
    for (key, value) in payment_tags(extra(PAYMENT_METHODS_KEYS).as_deref()) {
        push_tag(&mut tags, key, value);
    }
    for (key, value) in osm_tag_passthrough(submission) {
        push_tag(&mut tags, key.strip_prefix("osm:").unwrap_or(key), value);
    }
    // A submission only exists because the merchant accepts Bitcoin, and this
    // is the one tag a tagger must not forget.
    push_tag(&mut tags, "currency:XBT", "yes".to_string());

    tags.iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Two sections: a human-friendly summary a reviewer can read at a glance, and
/// a copy-pasteable block of OSM tags. The `Id:`/`Origin:` header and the
/// tagging-guidelines closing line stay put — the triage tooling parses them.
fn build_issue_body(submission: &PlaceSubmission) -> String {
    let human_section = build_human_section(submission);
    let osm_tags = format!("```\n{}\n```", build_osm_tags(submission));
    let body = format!(
        r#"
        Id: {id}
        Origin: {origin}

        {human_section}

        {osm_tags}

        OpenStreetMap viewer link: https://www.openstreetmap.org/#map=21/{lat}/{lon}

        OpenStreetMap editor link: https://www.openstreetmap.org/edit#map=21/{lat}/{lon}

        Please verify this information in line with our [tagging guidelines]({TAGGING_GUIDELINES_URL}) before adding to OSM.
    "#,
        id = submission.id,
        origin = submission.origin,
        lat = submission.lat,
        lon = submission.lon,
    );
    body.lines()
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join("\n")
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

            let body = build_issue_body(submission);
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
        "fetched revoked submissions with tickets",
    );

    let mut revocations_processed = 0;

    for submission in &revoked_submissions {
        if process_revoked_submission(submission, pool).await? {
            revocations_processed += 1;
        }
    }

    Ok(Res {
        issues_pending: submissions.len() as i64 - issues_closed,
        issues_created,
        issues_closed,
        revocations_processed,
    })
}

async fn process_revoked_submission(
    submission: &db::main::place_submission::schema::PlaceSubmission,
    pool: &Pool,
) -> Result<bool> {
    let Some(ticket_url) = submission.ticket_url.as_deref() else {
        warn!(
            submission_id = submission.id,
            "revoked submission has no ticket url"
        );
        return Ok(false);
    };

    let Some(issue) = service::gitea::get_issue(ticket_url.to_string(), pool).await? else {
        // The ticket is gone, so there is nothing left to act on: stop retrying
        // this submission on every run.
        warn!(
            submission_id = submission.id,
            ticket_url = ticket_url,
            "revoked submission's gitea ticket not found (404)"
        );
        mark_revocation_processed(submission.id, pool).await?;
        return Ok(false);
    };

    if issue
        .labels
        .iter()
        .any(|label| label.id == LOCATION_REMOVAL_LABEL_ID)
    {
        // Our own label (or a human's) says this ticket has already been dealt
        // with, so there is nothing to do here.
        mark_revocation_processed(submission.id, pool).await?;
        return Ok(false);
    }

    // Decide once and record the decision: on a retry the ticket state can
    // already reflect this function's own half-finished work, so it can no
    // longer be read to find out which action is the right one.
    let action = match submission.revocation_action {
        Some(action) => action,
        None => {
            let Some(action) = decide_revocation_action(&issue.state) else {
                warn!(
                    submission_id = submission.id,
                    ticket_url = ticket_url,
                    state = issue.state,
                    "unexpected gitea ticket state for revoked submission"
                );
                return Ok(false);
            };
            db::main::place_submission::queries::set_revocation_action(submission.id, action, pool)
                .await?;
            action
        }
    };

    match action {
        RevocationAction::Close => service::gitea::close_issue(ticket_url, pool).await?,
        RevocationAction::Reopen => service::gitea::reopen_issue(ticket_url, pool).await?,
    }

    let removal_labels = build_removal_labels(&submission.origin, pool).await;
    service::gitea::set_issue_labels(ticket_url, removal_labels, pool).await?;

    // Record completion before the comment: the comment is informational, and a
    // comment duplicated on every retry would be worse than a missing one.
    mark_revocation_processed(submission.id, pool).await?;

    if action == RevocationAction::Close {
        if let Err(e) = service::gitea::add_issue_comment(
            ticket_url,
            "This location was revoked before being processed.",
            pool,
        )
        .await
        {
            warn!(
                submission_id = submission.id,
                ticket_url = ticket_url,
                error = %e,
                "failed to add gitea comment for revoked submission"
            );
        }
    }

    Ok(true)
}

/// #59: a ticket that is still open was never processed, so a revocation cancels
/// it; one that is already closed means the place was processed, so the
/// revocation has to be raised as a removal request. An unknown state is left
/// alone and retried on the next run.
fn decide_revocation_action(state: &str) -> Option<RevocationAction> {
    match state {
        "open" => Some(RevocationAction::Close),
        "closed" => Some(RevocationAction::Reopen),
        _ => None,
    }
}

async fn mark_revocation_processed(id: i64, pool: &Pool) -> Result<()> {
    db::main::place_submission::queries::set_revocation_processed_at(
        id,
        Some(OffsetDateTime::now_utc()),
        pool,
    )
    .await?;
    Ok(())
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
    use super::{build_human_section, build_issue_body, build_issue_title, build_osm_tags};
    use crate::db::main::area::schema::Area;
    use crate::db::main::place_submission::schema::{PlaceSubmission, RevocationAction};
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

    fn submission(name: &str, category: &str, extra_fields: &[(&str, &str)]) -> PlaceSubmission {
        PlaceSubmission {
            id: 18899,
            origin: "square".into(),
            external_id: "15".into(),
            lat: 17.8960777,
            lon: 101.6562147,
            category: category.into(),
            name: name.into(),
            extra_fields: extra_fields
                .iter()
                .map(|(key, value)| (key.to_string(), Value::String(value.to_string())))
                .collect(),
            ticket_url: None,
            revoked: false,
            revocation_action: None,
            revocation_processed_at: None,
            submitted_by: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            closed_at: None,
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
    fn human_section_renders_submitted_fields_in_groups() {
        let submission = submission(
            "มะพร้าวแก้ว วรรัตน์",
            "shopping",
            &[
                ("name:en", "WORARAT COCO"),
                ("address", "Chai Khong Road, 42110 Chiang Khan"),
                ("phone", "0865001593"),
                ("opening_hours", "Mo-Th 16:00-21:00; Fr-Su 00:00-24:00"),
                ("payment_methods", "onchain,lightning,nfc"),
                ("notes", "ของดีเมืองเชียงคาน ID Line/ Air1070"),
                (
                    "osm_edit_url",
                    "https://www.openstreetmap.org/edit#map=19/17.8960777/101.6562147",
                ),
            ],
        );

        assert_eq!(
            build_human_section(&submission),
            "\
Name (local): มะพร้าวแก้ว วรรัตน์
Name (English): WORARAT COCO
Category: shopping

Address: Chai Khong Road, 42110 Chiang Khan
Phone: 0865001593
Opening hours: Mo-Th 16:00-21:00; Fr-Su 00:00-24:00

Payment methods: onchain, lightning, nfc

Notes: ของดีเมืองเชียงคาน ID Line/ Air1070

Additional fields:
osm_edit_url: https://www.openstreetmap.org/edit#map=19/17.8960777/101.6562147"
        );
    }

    #[test]
    fn human_section_omits_lines_without_data() {
        let submission = submission(
            "TRIPLE DIAMONDS IN THE ROUGH LLC",
            "accounting",
            &[
                (
                    "address",
                    "4323 Division St Ste 102 Metairie LA 70002-3179 US",
                ),
                ("opening_hours", "Fr-Sa 07:00-18:30; Su-Th 07:00-18:00"),
                ("last_updated", "2026-09-22T13:58:09.138624029Z"),
            ],
        );

        assert_eq!(
            build_human_section(&submission),
            "\
Name (local): TRIPLE DIAMONDS IN THE ROUGH LLC
Category: accounting

Address: 4323 Division St Ste 102 Metairie LA 70002-3179 US
Opening hours: Fr-Sa 07:00-18:30; Su-Th 07:00-18:00

Additional fields:
last_updated: 2026-09-22T13:58:09.138624029Z"
        );
    }

    #[test]
    fn human_section_collapses_values_to_a_single_line() {
        let submission = submission(
            "Multi Line",
            "cafe",
            &[("notes", "first line\n\n  second line\tthird")],
        );

        assert!(build_human_section(&submission).contains("Notes: first line second line third"));
    }

    #[test]
    fn human_section_skips_blank_and_null_values() {
        let mut submission = submission("Sparse", "cafe", &[("phone", "   ")]);
        submission
            .extra_fields
            .insert("website".into(), Value::Null);

        assert_eq!(
            build_human_section(&submission),
            "\
Name (local): Sparse
Category: cafe"
        );
    }

    #[test]
    fn osm_tags_map_submitted_fields() {
        let submission = submission(
            "มะพร้าวแก้ว วรรัตน์",
            "shopping",
            &[
                ("name:en", "WORARAT COCO"),
                ("address", "Chai Khong Road, 42110 Chiang Khan"),
                ("phone", "0865001593"),
                ("opening_hours", "Mo-Th 16:00-21:00; Fr-Su 00:00-24:00"),
                ("website", "https://worarat.example.com"),
                ("contact", "worarat@example.com"),
                ("description", "Coconut sweets"),
                ("payment_methods", "onchain,lightning,nfc"),
            ],
        );

        assert_eq!(
            build_osm_tags(&submission),
            "\
name=มะพร้าวแก้ว วรรัตน์
name:en=WORARAT COCO
addr:full=Chai Khong Road, 42110 Chiang Khan
phone=0865001593
opening_hours=Mo-Th 16:00-21:00; Fr-Su 00:00-24:00
website=https://worarat.example.com
email=worarat@example.com
description=Coconut sweets
payment:onchain=yes
payment:lightning=yes
payment:lightning_contactless=yes
currency:XBT=yes"
        );
    }

    #[test]
    fn osm_tags_only_include_name_and_currency_for_sparse_submission() {
        let submission = submission("Satoshi Cafe", "cafe", &[]);

        assert_eq!(
            build_osm_tags(&submission),
            "\
name=Satoshi Cafe
currency:XBT=yes"
        );
    }

    #[test]
    fn osm_tags_drop_unknown_payment_methods_and_deduplicate() {
        let submission = submission(
            "Satoshi Cafe",
            "cafe",
            &[("payment_methods", "lightning, LNbits , lightning;nfc")],
        );

        assert_eq!(
            build_osm_tags(&submission),
            "\
name=Satoshi Cafe
payment:lightning=yes
payment:lightning_contactless=yes
currency:XBT=yes"
        );
    }

    #[test]
    fn osm_tags_describe_non_email_contact_as_contact() {
        let submission = submission("Satoshi Cafe", "cafe", &[("contact", "@satoshicafe")]);

        assert!(build_osm_tags(&submission).contains("contact=@satoshicafe"));
    }

    #[test]
    fn body_keeps_id_origin_and_tagging_guidelines_line() {
        let submission = submission("Satoshi Cafe", "cafe", &[("address", "1 Main St")]);
        let body = build_issue_body(&submission);

        assert!(body
            .trim_start()
            .starts_with("Id: 18899\nOrigin: square\n\nName (local): Satoshi Cafe"));
        assert!(body.contains("Name (local): Satoshi Cafe"));
        assert!(body.contains("```\nname=Satoshi Cafe\naddr:full=1 Main St\ncurrency:XBT=yes\n```"));
        assert!(body.contains(
            "OpenStreetMap viewer link: https://www.openstreetmap.org/#map=21/17.8960777/101.6562147"
        ));
        assert!(body.contains(
            "Please verify this information in line with our [tagging guidelines](https://gitea.btcmap.org/teambtcmap/btcmap-general/wiki/Tagging-Merchants) before adding to OSM."
        ));
        assert!(!body.contains("To verify this imported place:"));
        assert!(!body.contains("Extra fields:"));
    }

    #[test]
    fn osm_tags_pass_through_osm_prefixed_fields() {
        let submission = submission(
            "Satoshi Cafe",
            "cafe",
            &[
                ("osm:addr:housenumber", "938"),
                ("osm:addr:street", "Ocean Blvd"),
                ("osm:addr:city", "Coronado"),
                ("osm:payment:onchain", "yes"),
                ("osm:payment:lightning", "yes"),
            ],
        );

        assert_eq!(
            build_osm_tags(&submission),
            "\
name=Satoshi Cafe
addr:city=Coronado
addr:housenumber=938
addr:street=Ocean Blvd
payment:lightning=yes
payment:onchain=yes
currency:XBT=yes"
        );
    }

    #[test]
    fn osm_tags_prefer_mapped_fields_over_osm_passthrough() {
        let submission = submission(
            "Satoshi Cafe",
            "cafe",
            &[
                ("payment_methods", "onchain"),
                ("osm:payment:onchain", "no"),
            ],
        );

        assert_eq!(
            build_osm_tags(&submission),
            "\
name=Satoshi Cafe
payment:onchain=yes
currency:XBT=yes"
        );
    }

    #[test]
    fn human_section_omits_passed_through_osm_tags() {
        let submission = submission(
            "Satoshi Cafe",
            "cafe",
            &[
                ("osm:addr:city", "Coronado"),
                (
                    "osm_edit_url",
                    "https://www.openstreetmap.org/edit#map=19/17.8960777/101.6562147",
                ),
            ],
        );

        assert_eq!(
            build_human_section(&submission),
            "\
Name (local): Satoshi Cafe
Category: cafe

Additional fields:
osm_edit_url: https://www.openstreetmap.org/edit#map=19/17.8960777/101.6562147"
        );
    }

    #[actix_web::test]
    async fn build_removal_labels_falls_back_to_default() {
        let pool = pool();
        let labels = super::build_removal_labels("unknown-origin", &pool).await;
        assert_eq!(labels, vec![super::LOCATION_REMOVAL_LABEL_ID]);
    }

    #[test]
    fn decide_revocation_action_follows_the_ticket_state() {
        assert_eq!(
            Some(RevocationAction::Close),
            super::decide_revocation_action("open")
        );
        assert_eq!(
            Some(RevocationAction::Reopen),
            super::decide_revocation_action("closed")
        );
        assert_eq!(None, super::decide_revocation_action("merged"));
    }
}
