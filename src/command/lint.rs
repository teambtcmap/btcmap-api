use std::collections::HashMap;
use std::env;

use crate::command::generate_android_icons::android_icon;
use crate::command::generate_report;
use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use rusqlite::named_params;
use time::macros::format_description;
use time::Date;
use tracing::error;
use tracing::info;

pub async fn run(db: Connection) -> Result<()> {
    info!("Started linting");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            element::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Element>, _>>()?
        .into_iter()
        .filter(|it| it.deleted_at.len() == 0)
        .collect();

    info!(
        elements = elements.len(),
        "Loaded all elements from database"
    );

    let date_format = format_description!("[year]-[month]-[day]");

    for element in elements {
        let url = format!("https://openstreetmap.org/{}", element.id.replace(":", "/"));

        let survey_date = element.osm_json["tags"]["survey:date"]
            .as_str()
            .unwrap_or("");

        if survey_date.len() > 0 {
            let parsed_date = Date::parse(survey_date, &date_format);

            if parsed_date.is_err() {
                let message = format!(
                    "{} survey:date is not formatted properly: {}",
                    url, survey_date,
                );
                error!(message);
                send_discord_message(message).await;
            }
        }

        let check_date = element.osm_json["tags"]["check_date"]
            .as_str()
            .unwrap_or("");

        if check_date.len() > 0 {
            let parsed_date = Date::parse(check_date, &date_format);

            if parsed_date.is_err() {
                let message = format!(
                    "{} check_date is not formatted properly: {}",
                    url, check_date,
                );
                error!(message);
                send_discord_message(message).await;
            }
        }

        let check_date_currency_xbt = element.osm_json["tags"]["check_date:currency:XBT"]
            .as_str()
            .unwrap_or("");

        if check_date_currency_xbt.len() > 0 {
            let parsed_date = Date::parse(check_date_currency_xbt, &date_format);

            if parsed_date.is_err() {
                let message = format!(
                    "{} check_date:currency:XBT is not formatted properly: {}",
                    url, check_date_currency_xbt,
                );
                error!(message);
                send_discord_message(message).await;
            }
        }

        let payment_lighting = element.osm_json["tags"]["payment:lighting"]
            .as_str()
            .unwrap_or("");

        if payment_lighting.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting");
        }

        let payment_lightning_contacless = element.osm_json["tags"]["payment:lightning_contacless"]
            .as_str()
            .unwrap_or("");

        if payment_lightning_contacless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lightning_contacless");
        }

        let payment_lighting_contactless = element.osm_json["tags"]["payment:lighting_contactless"]
            .as_str()
            .unwrap_or("");

        if payment_lighting_contactless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting_contactless");
        }

        let currency_xbt = element.osm_json["tags"]["currency:XBT"]
            .as_str()
            .unwrap_or("");

        let payment_bitcoin = element.osm_json["tags"]["payment:bitcoin"]
            .as_str()
            .unwrap_or("");

        if currency_xbt == "yes" && payment_bitcoin == "yes" {
            let message = format!(
                "{} Both currency:XBT and payment:bitcoin are set to \"yes\"",
                url,
            );
            error!(message);
            send_discord_message(message).await;
        }

        if payment_bitcoin == "yes" && survey_date != "" {
            let message = format!(
                "{} Legacy payment:bitcoin tag is present but survey:date is available, worth upgrading to currency:XBT",
                url,
            );
            error!(message);
            send_discord_message(message).await;
        }

        if payment_bitcoin == "yes" && check_date != "" {
            let message = format!(
                "{} Legacy payment:bitcoin tag is present but check_date is available, worth upgrading to currency:XBT",
                url,
            );
            error!(message);
            send_discord_message(message).await;
        }

        if payment_bitcoin == "yes" && check_date_currency_xbt != "" {
            let message = format!(
                "{} Legacy payment:bitcoin tag is present but check_date:currency:XBT is available, worth upgrading to currency:XBT",
                url,
            );
            error!(message);
            send_discord_message(message).await;
        }

        if generate_report::up_to_date(&element.osm_json)
            && android_icon(&element.osm_json["tags"].as_object().unwrap()) == "question_mark"
        {
            let message = format!("{} Up-to-date element has no icon", url,);
            error!(message);
            send_discord_message(message).await;
        }
    }

    info!("Finished linting");

    Ok(())
}

async fn send_discord_message(text: String) {
    if let Ok(discord_webhook_url) = env::var("LINT_DISCORD_WEBHOOK_URL") {
        info!("Sending Discord message");
        let mut args = HashMap::new();
        args.insert("username", "btcmap.org".to_string());
        args.insert("content", text);

        let response = reqwest::Client::new()
            .post(discord_webhook_url)
            .json(&args)
            .send()
            .await;

        match response {
            Ok(response) => {
                info!(status = ?response.status(), "Got response");
            }
            Err(_) => {
                error!("Failed to send Discord message");
            }
        }
    }
}
