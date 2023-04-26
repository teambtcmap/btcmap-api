use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{error, info};

use crate::Result;
use std::io;

#[derive(Debug, Serialize, Deserialize)]
struct LogEntry {
    #[serde(with = "time::serde::iso8601")]
    timestamp: time::OffsetDateTime,
    level: String,
    target: String,
    fields: Map<String, Value>,
}

pub async fn run() -> Result<()> {
    let mut entries: Vec<LogEntry> = Vec::new();

    loop {
        let mut input = String::new();

        io::stdin().read_line(&mut input).unwrap();
        input = input.trim().to_string();

        if input == "" {
            break;
        }

        let entry: serde_json::Result<LogEntry> = serde_json::from_str(&input);

        match entry {
            Ok(entry) => entries.push(entry),
            Err(e) => error!(entry_body = input, ?e),
        }
    }

    let period_start = entries.first().unwrap().timestamp.clone();
    let period_end = entries.last().unwrap().timestamp.clone();
    let period_duration = period_end - period_start;
    let log_entries_per_second = format!(
        "{:.2}",
        entries.len() as f64 / period_duration.as_seconds_f64()
    );

    info!(start = ?period_start, end = ?period_end, period_seconds = ?period_duration.as_seconds_f64(), log_entries = entries.len(), log_entries_per_second);

    Ok(())
}
