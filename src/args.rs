use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// The location the output file should be saved to.
    #[arg(default_value = "output.json")]
    pub output_file: PathBuf,
    /// The servers to query.
    #[arg(long, value_parser, value_delimiter = ',', required = true)]
    pub servers: Vec<String>,
    /// The interval between queries in seconds.
    #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u16).range(1..))]
    pub interval: u16,
    /// The failure tolerance level.
    #[arg(long, default_value = "one")]
    pub failure_tolerance: FailureTolerance,
    /// How many triggers to wait before retrying a failed server
    #[arg(long, default_value = "1")]
    pub failure_retry_wait: u16,
}

/// The different failure tolerance levels.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FailureTolerance {
    /// All servers must respond.
    None,
    /// One server failing to respond is tolerated.
    One,
    /// All servers failing to respond is tolerated.
    All,
}
