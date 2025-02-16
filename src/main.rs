use args::{Args, FailureTolerance};
use chrono::{DateTime, Utc};
use clap::Parser;
use fetch::{query_server, ServerInfo};
use serde::Serialize;

mod args;
mod fetch;

#[derive(Serialize)]
struct ServerInfoFetcherResponse {
    pub servers: Vec<ServerInfoOutput>,
    pub last_update: DateTime<Utc>,
}

#[derive(Serialize)]
struct ServerInfoOutput {
    pub server: Option<ServerInfo>,
    pub retry_wait: u16,
    #[serde(skip)]
    pub server_address: String
}

impl ServerInfoOutput {
    fn new(addr: String) -> ServerInfoOutput {
        ServerInfoOutput {
            server: None,
            retry_wait: 0,
            server_address: addr
        }
    }
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

    let mut output = ServerInfoFetcherResponse {
        servers: servers.iter().map(|addr| ServerInfoOutput::new(addr.to_string())).collect(),
        last_update: Utc::now(),
    };

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval as u64));
    loop {
        let mut errors = 0;
        for server_output in &mut output.servers {
            if server_output.retry_wait != 0 {
                server_output.retry_wait -= 1;
                continue;
            }

            let server_info = query_server(&server_output.server_address).await;
            if let Err(error) = &server_info {
                eprintln!("Error querying server {}: {}", server_output.server_address, error);
                match failure_tolerance {
                    FailureTolerance::None => {
                        eprintln!("Exiting due to failure tolerance violation.");
                        return;
                    }
                    FailureTolerance::One if errors != 0 => {
                        eprintln!("Exiting due to failure tolerance violation.");
                        return;
                    }
                    _ => {
                        if failure_retry_wait != 0 {
                            server_output.retry_wait = failure_retry_wait;
                        }
                        errors += 1;
                    }
                };
                server_output.server = None;
            } else {
                if let Ok(parsed_info) = server_info {
                    server_output.server = Some(parsed_info);
                } else {
                    eprintln!("Server at {} sent a malformed status response.", server_output.server_address);
                    return;
                }
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
