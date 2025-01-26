use std::collections::HashMap;

pub async fn post_message(webhook_url: impl Into<String>, message: impl Into<String>) {
    let url = webhook_url.into();
    if url.is_empty() {
        return;
    }
    let message = message.into();
    let mut args = HashMap::new();
    args.insert("username", "btcmap.org");
    args.insert("content", &message);
    let _ = reqwest::Client::new().post(url).json(&args).send().await;
}
