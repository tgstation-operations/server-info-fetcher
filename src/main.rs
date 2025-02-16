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
    pub data: Option<ServerInfo>,
    pub identifier: Option<String>,
    pub retry_wait: u16,
    #[serde(skip)]
    pub address: String,
}

impl ServerInfoOutput {
    fn new(address: String) -> ServerInfoOutput {
        ServerInfoOutput {
            data: None,
            identifier: None,
            retry_wait: 0,
            address,
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
        servers: servers
            .iter()
            .map(|addr| ServerInfoOutput::new(addr.to_string()))
            .collect(),
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

            let server_info = query_server(&server_output.address).await;
            if let Err(error) = &server_info {
                eprintln!("Error querying server {}: {}", server_output.address, error);
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
                server_output.data = None;
            } else {
                let parsed_info = server_info.expect("should never happen");
                match server_output.identifier {
                    None => server_output.identifier = Some(parsed_info.identifier.clone()),
                    Some(ref identifier) if identifier.ne(&parsed_info.identifier) => {
                        eprintln!(
                            "Server {} changed identifier from `{}` to `{}`",
                            server_output.address, identifier, parsed_info.identifier
                        );
                        // we don't really care about this, but it should be logged
                        server_output.identifier = Some(parsed_info.identifier.clone());
                    }
                    _ => {}
                }
                server_output.data = Some(parsed_info);
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
