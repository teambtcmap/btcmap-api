use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ChatMessage {
    pub content: String,
}

#[derive(Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

#[derive(Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,
}

pub static MINIMAX_M2_5: &str = "minimax/minimax-m2.5";

pub async fn chat(prompt: String, model: &str, pool: &Pool) -> Result<String> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.ppq_key.is_empty() {
        Err("ppq ai key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
    });
    let response = client
        .post("https://api.ppq.ai/chat/completions")
        .header("Authorization", format!("Bearer {}", conf.ppq_key))
        .header("Content-Type", "application/json")
        .json(&args)
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("ppq.ai API error ({}): {}", status, text).into());
    }
    let response: ChatCompletionResponse = response.json().await?;
    let content = response
        .choices
        .first()
        .ok_or("no response choices")?
        .message
        .content
        .clone();
    Ok(content)
}
