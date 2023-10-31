use std::collections::HashMap;
use std::env;

use crate::model::Element;
use crate::Connection;
use crate::Result;
use time::macros::format_description;
use time::Date;
use tracing::debug;
use tracing::error;
use tracing::info;

pub async fn run(conn: Connection) -> Result<()> {
    debug!("Started linting");

    let elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();

    debug!(
        elements = elements.len(),
        "Loaded all elements from database"
    );

    let date_format = format_description!("[year]-[month]-[day]");

    for element in elements {
        let url = format!(
            "https://openstreetmap.org/{}/{}",
            element.overpass_data.r#type, element.overpass_data.id,
        );

        let survey_date = element.overpass_data.tag("survey:date");

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

        let check_date = element.overpass_data.tag("check_date");

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

        let check_date_currency_xbt = element.overpass_data.tag("check_date:currency:XBT");

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

        let payment_lighting = element.overpass_data.tag("payment:lighting");

        if payment_lighting.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting");
        }

        let payment_lightning_contacless =
            element.overpass_data.tag("payment:lightning_contacless");

        if payment_lightning_contacless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lightning_contacless");
        }

        let payment_lighting_contactless =
            element.overpass_data.tag("payment:lighting_contactless");

        if payment_lighting_contactless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting_contactless");
        }

        let currency_xbt = element.overpass_data.tag("currency:XBT");

        let payment_bitcoin = element.overpass_data.tag("payment:bitcoin");

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

        if element.tag("icon:android").as_str().unwrap_or_default() == ""
            || element.tag("icon:android").as_str().unwrap_or_default() == "question_mark"
        {
            let message = format!("{} Icon is missing", url);
            error!(message);
            send_discord_message(message).await;
        }

        if element.overpass_data.verification_date().is_none() {
            let message = format!("{} Not verified", url);
            error!(message);
            send_discord_message(message).await;
        }

        if element.overpass_data.verification_date().is_some()
            && !element.overpass_data.up_to_date()
        {
            let message = format!("{} Out of date", url);
            error!(message);
            send_discord_message(message).await;
        }
    }

    debug!("Finished linting");

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
