use std::{collections::HashMap, env, fmt::Write};

use tracing::{
    error,
    field::{Field, Visit},
    info,
};

pub struct DiscordLayer;

impl<S> tracing_subscriber::Layer<S> for DiscordLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        for field in event.fields() {
            if field.name() == "discord_message" {
                let mut message = "".to_string();
                let mut visitor = DiscordMessageVisitor {
                    message: &mut message,
                };
                event.record(&mut visitor);
                if let Ok(url) = env::var("DISCORD_WEBHOOK_URL") {
                    tokio::runtime::Handle::current()
                        .spawn(async move { send_discord_message(message, &url).await });
                }
            }

            if field.name() == "admin_channel_message" {
                let mut message = "".to_string();
                let mut visitor = DiscordMessageVisitor {
                    message: &mut message,
                };
                event.record(&mut visitor);
                if let Ok(url) = env::var("DISCORD_ADMIN_CHANNEL_WEBHOOK_URL") {
                    tokio::runtime::Handle::current()
                        .spawn(async move { send_discord_message(message, &url).await });
                }
            }
        }
    }
}

struct DiscordMessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> Visit for DiscordMessageVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "discord_message" {
            write!(self.message, "{}", value).unwrap();
        }

        if field.name() == "admin_channel_message" {
            write!(self.message, "{}", value).unwrap();
        }
    }
}

async fn send_discord_message(message: String, webhook_url: &str) {
    let mut args = HashMap::new();
    args.insert("username", "btcmap.org".to_string());
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
