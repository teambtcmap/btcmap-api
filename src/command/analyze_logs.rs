use serde::Serialize;

use crate::Result;
use std::io;

#[derive(Debug, Serialize)]
struct LogEntry {
    level: String,
    component: String,
    ip: String,
    first_request_line: String,
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

        let mut level = String::new();
        let mut parsed_level = false;

        let mut component = String::new();
        let mut parsed_component = false;

        let mut ip = String::new();
        let mut ip_parsed = false;

        let mut first_request_line = String::new();
        let mut first_request_line_parsed = false;

        for char in input.chars() {
            if !parsed_level {
                match char {
                    '[' => {}
                    ' ' => parsed_level = true,
                    _ => level.push(char),
                }

                continue;
            }

            if !parsed_component {
                match char {
                    ']' => parsed_component = true,
                    _ => component.push(char),
                }

                continue;
            }

            if !ip_parsed {
                match char {
                    '"' => ip_parsed = true,
                    _ => ip.push(char),
                }

                continue;
            }

            if !first_request_line_parsed {
                match char {
                    '"' => first_request_line_parsed = true,
                    _ => first_request_line.push(char),
                }    
            }
        }

        component = component.trim().into();
        ip = ip.trim().into();

        if component == "actix_web::middleware::logger" {
            entries.push(LogEntry { level: level, component: component, ip: ip, first_request_line: first_request_line });
        }
    }

    //println!("{}", serde_json::to_string_pretty(&entries).unwrap());

    println!("Total requests served: {}", entries.len());
    println!("Requests per second: {:.2}", entries.len() as f64 / 86400.0);

    Ok(())
}
