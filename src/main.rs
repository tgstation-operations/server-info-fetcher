use std::collections::HashMap;

use args::{Args, FailureTolerance};
use chrono::{DateTime, Utc};
use clap::Parser;
use fetch::{query_server, ServerInfo};
use serde::Serialize;

mod args;
mod fetch;

#[derive(Serialize)]
struct ServerInfoOutput {
    pub servers: HashMap<String, ServerInfo>,
    pub retry_waits: HashMap<String, u16>,
    pub last_update: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let interval = args.interval;
    let output_file = args.output_file;
    let servers = args.servers;
    let failure_tolerance = args.failure_tolerance;
    let failure_retry_wait = args.failure_retry_wait;

    if servers.is_empty() {
        eprintln!("No servers specified!");
        return;
    }

    let mut output = ServerInfoOutput {
        servers: HashMap::new(),
        retry_waits: HashMap::new(),
        last_update: Utc::now(),
    };
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval as u64));
    loop {
        let mut errors = 0;
        for server in &servers {
            if let Some(retry_wait) = output.retry_waits.get_mut(server) {
                if *retry_wait != 0 {
                    *retry_wait -= 1;
                    continue;
                }
            }

            let server_info = query_server(server).await;
            if let Err(error) = &server_info {
                eprintln!("Error querying server {}: {}", server, error);
                match failure_tolerance {
                    FailureTolerance::None => {
                        eprintln!("Exiting due to failure tolerance violation.");
                        return;
                    }
                    FailureTolerance::One => {
                        if errors != 0 {
                            eprintln!("Exiting due to failure tolerance violation.");
                        }
                        return;
                    }
                    _ => {
                        if failure_retry_wait != 0 {
                            output.retry_waits.insert(server.to_string(), failure_retry_wait);
                        }
                        errors += 1;
                    }
                }
            } else {
                let server_info = server_info.expect("Could not parse server info");
                let address = match &server_info.public_address {
                    Some(s) => s,
                    None => server
                };
                output.servers.insert(address.to_string(), server_info);
            }
        }

        if errors == servers.len() {
            eprintln!("All servers failed to respond.");
            return;
        }

        if errors != 0 {
            eprintln!("{} servers failed to respond.", errors);
        }

        output.last_update = Utc::now();
        let json = serde_json::to_string(&output).unwrap();
        if let Err(error) = tokio::fs::write(&output_file, json).await {
            eprintln!("Error writing to output file: {}", error);
            return;
        }

        interval.tick().await;
    }
}
