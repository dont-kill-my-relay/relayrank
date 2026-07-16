use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use relay_rank::{network_metric, relay_metric, tor_status::Consensus};

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
    RelayMetric {
        exclusion_list: Option<PathBuf>,
    },
    BuildInference,
    NetworkMetric {
        mapping_file: PathBuf,
        as_path_file: PathBuf,
        exclusion_list: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    let consensus = Consensus::new(&args.cache, args.datetime)?;
    match args.command {
        Command::BuildInference => todo!("build inference"),
        Command::NetworkMetric {
            mapping_file,
            as_path_file,
            exclusion_list,
        } => network_metric::compute(&consensus, &mapping_file, &as_path_file, &exclusion_list),
        Command::RelayMetric { exclusion_list } => {
            relay_metric::compute(&consensus, &exclusion_list)
        }
    }
}
