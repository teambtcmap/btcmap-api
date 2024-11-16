use std::{collections::HashMap, env};
use tracing::{error, info};

pub static CHANNEL_OSM_CHANGES: &str = "DISCORD_WEBHOOK_URL";
pub static CHANNEL_API: &str = "DISCORD_ADMIN_CHANNEL_WEBHOOK_URL";

pub async fn send_message(channel: &str, message: impl Into<String>) {
    let message = message.into();
    info!("attempting to send message {message} to channel {channel}");
    if let Ok(webhook_url) = env::var(channel) {
        _send_message(&message, &webhook_url).await;
    }
}

pub async fn send_message_to_channel(message: &str, channel: &str) {
    info!("attempting to send message {message} to channel {channel}");
    if let Ok(webhook_url) = env::var(channel) {
        _send_message(message, &webhook_url).await;
    }
}

async fn _send_message(message: &str, webhook_url: &str) {
    let mut args = HashMap::new();
    args.insert("username", "btcmap.org");
    args.insert("content", message);

    info!("Sending discord message");

    let response = reqwest::Client::new()
        .post(webhook_url)
        .json(&args)
        .send()
        .await;

    match response {
        Ok(response) => {
            info!(response_status = ?response.status(), "Got response");
        }
        Err(_) => {
            error!("Failed to send Discord message");
        }
    };
}
