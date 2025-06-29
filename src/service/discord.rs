use crate::conf::Conf;
use std::collections::HashMap;
use strum::Display;
use tracing::{error, info, warn};

const BOT_USERNAME: &str = "btcmap.org";

#[derive(Debug, Display)]
pub enum Channel {
    Api,
    OsmChanges,
}

pub fn send(message: impl Into<String>, channel: Channel, conf: &Conf) {
    let message = message.into();
    if message.is_empty() {
        warn!("Empty message provided, not sending");
        return;
    }

    let webhook_url = match channel {
        Channel::Api => conf.discord_webhook_api.clone(),
        Channel::OsmChanges => conf.discord_webhook_osm_changes.clone(),
    };

    if webhook_url.is_empty() {
        warn!(
            channel = channel.to_string(),
            "Webhook URL is not configured"
        );
        return;
    }

    actix_web::rt::spawn(async move {
        let mut args = HashMap::new();
        args.insert("username", BOT_USERNAME);
        args.insert("content", &message);

        info!("Sending message to webhook (async)");
        let response = reqwest::Client::new()
            .post(&webhook_url)
            .json(&args)
            .send()
            .await;

        match response {
            Ok(response) if !response.status().is_success() => {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".into());
                error!(
                    status = status.to_string(),
                    error_text, "Webhook request failed"
                );
            }
            Err(e) => {
                error!(e = e.to_string(), "Webhook request failed");
            }
            _ => info!("Message successfully sent (async)"),
        }
    });
}
