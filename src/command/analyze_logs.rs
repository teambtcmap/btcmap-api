use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{error, info};

use crate::Result;
use std::{
    collections::{BTreeMap, HashMap},
    io,
};

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

    let period_start = entries.first().unwrap().timestamp;
    let period_end = entries.last().unwrap().timestamp;
    let period_duration = period_end - period_start;
    let log_entries_per_second = format!(
        "{:.2}",
        entries.len() as f64 / period_duration.as_seconds_f64()
    );
    let http_requests: Vec<&LogEntry> = entries
        .iter()
        .filter(|it| it.fields.contains_key("req_method"))
        .collect();
    let http_requests_per_second = format!(
        "{:.2}",
        http_requests.len() as f64 / period_duration.as_seconds_f64()
    );

    let mut rec_count_by_path: HashMap<&str, i128> = HashMap::new();

    for req in &http_requests {
        let path = req.fields["req_path"].as_str().unwrap();

        if rec_count_by_path.contains_key(path) {
            let old_count = rec_count_by_path.get(path).unwrap();
            rec_count_by_path.insert(path, old_count + 1);
        } else {
            rec_count_by_path.insert(path, 1);
        }
    }

    let rec_count_by_path: BTreeMap<_, _> = rec_count_by_path.iter().map(|(k, v)| (v, k)).collect();
    let mut most_frequent_req: Vec<_> = rec_count_by_path.keys().into_iter().collect();
    most_frequent_req.reverse();
    let most_frequent_req: Vec<_> = most_frequent_req
        .iter()
        .map(|it| (rec_count_by_path.get(*it).unwrap(), it))
        .collect();
    let most_freq_req = serde_json::to_string(&most_frequent_req).unwrap();

    let mut res_times_seconds: Vec<f64> = http_requests
        .iter()
        .map(|it| it.fields["res_time_sec"].as_f64().unwrap())
        .collect();
    res_times_seconds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean_res_time_seconds = mean(&res_times_seconds);
    let median_res_time_seconds = median(&res_times_seconds);
    let min_res_time_seconds = res_times_seconds.first().unwrap();
    let max_res_time_seconds = res_times_seconds.last().unwrap();

    info!(
        start = ?period_start,
        end = ?period_end, period_seconds =
        ?period_duration.as_seconds_f64(),
        log_entries = entries.len(),
        log_entries_per_second,
        http_requests = http_requests.len(),
        http_requests_per_second,
        most_freq_req,
        mean_res_time_seconds,
        median_res_time_seconds,
        min_res_time_seconds,
        max_res_time_seconds,
    );

    Ok(())
}

fn median(numbers: &Vec<f64>) -> f64 {
    let mid = numbers.len() / 2;

    if numbers.len() % 2 == 0 {
        mean(&vec![numbers[mid - 1], numbers[mid]])
    } else {
        numbers[mid]
    }
}

fn mean(numbers: &Vec<f64>) -> f64 {
    let sum: f64 = numbers.iter().sum();
    sum / numbers.len() as f64
}
