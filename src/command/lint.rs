use std::collections::HashMap;
use std::env;

use crate::model::Element;
use crate::Connection;
use crate::Result;
use time::macros::format_description;
use time::Date;
use tracing::error;
use tracing::info;

pub async fn run(conn: Connection) -> Result<()> {
    info!("Started linting");

    let elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == "")
        .collect();

    info!(
        elements = elements.len(),
        "Loaded all elements from database"
    );

    let date_format = format_description!("[year]-[month]-[day]");

    for element in elements {
        let url = format!("https://openstreetmap.org/{}", element.id.replace(":", "/"));

        let survey_date = element.get_osm_tag_value("survey:date");

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

        let check_date = element.get_osm_tag_value("check_date");

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

        let check_date_currency_xbt = element.get_osm_tag_value("check_date:currency:XBT");

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

        let payment_lighting = element.get_osm_tag_value("payment:lighting");

        if payment_lighting.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting");
        }

        let payment_lightning_contacless =
            element.get_osm_tag_value("payment:lightning_contacless");

        if payment_lightning_contacless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lightning_contacless");
        }

        let payment_lighting_contactless =
            element.get_osm_tag_value("payment:lighting_contactless");

        if payment_lighting_contactless.len() > 0 {
            error!(element.id, "Spelling issue: payment:lighting_contactless");
        }

        let currency_xbt = element.get_osm_tag_value("currency:XBT");

        let payment_bitcoin = element.get_osm_tag_value("payment:bitcoin");

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

        if element.get_btcmap_tag_value_str("icon:android") == ""
            || element.get_btcmap_tag_value_str("icon:android") == "question_mark"
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
