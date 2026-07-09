use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};

mod network_metric;
mod relay_metric;
mod tor_status;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "./cache")]
    cache: PathBuf,
    datetime: DateTime<Utc>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    RelayMetric,
    BuildInference,
    NetworkMetric {
        mapping_file: PathBuf,
        as_path_file: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::BuildInference => todo!("build inference"),
        Command::NetworkMetric {
            mapping_file,
            as_path_file,
        } => network_metric::compute(&args.cache, args.datetime, &mapping_file, &as_path_file),
        Command::RelayMetric => todo!("relay metric"),
    }
}
