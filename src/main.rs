use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use hex::FromHex;

use crate::tor_status::{Consensus, RelayId};

mod network_metric;
mod relay_metric;
mod tor_status;

fn parse_exclusion_list(exclusion_list: &Path) -> Result<Vec<RelayId>> {
    let mut exclusion_file = File::open(exclusion_list)?;
    let mut content = String::new();
    exclusion_file.read_to_string(&mut content)?;

    content.lines().map(RelayId::from_hex).collect()
}

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
