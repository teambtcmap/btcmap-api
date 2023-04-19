use std::{fmt::Write, env, collections::HashMap};

use tracing::{field::{Visit, Field}, info, error};

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
                tokio::runtime::Handle::current().spawn_blocking(|| {
                    send_discord_message(message)
                });
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
    }
}

fn send_discord_message(message: String) {
    if let Ok(discord_webhook_url) = env::var("DISCORD_WEBHOOK_URL") {
        let mut args = HashMap::new();
        args.insert("username", "btcmap.org".to_string());
        args.insert("content", message);

        info!("Sending discord message");

        let response = reqwest::blocking::Client::new()
            .post(discord_webhook_url)
            .json(&args)
            .send();

        match response {
            Ok(response) => {
                info!(response_status = ?response.status(), "Got response");
            }
            Err(_) => {
                error!("Failed to send Discord message");
            }
        };
    }
}
