use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Ok, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use hex::FromHex;
use relay_rank::{
    network_metric, relay_metric,
    tor_status::{Consensus, Relay, RelayId},
};

fn parse_exclusion_list(exclusion_list: &Path) -> Result<Vec<RelayId>> {
    let mut exclusion_file = File::open(exclusion_list)?;
    let mut content = String::new();
    exclusion_file.read_to_string(&mut content)?;

    content.lines().map(RelayId::from_hex).collect()
}

fn rank(mut metric: Vec<(&Relay, f64)>, exclusion_list: Option<PathBuf>) -> Result<()> {
    metric.sort_by(|(_, m), (_, o)| m.total_cmp(o));
    metric.reverse();
    let iterator = metric.iter().enumerate().map(|(i, (r, m))| (i + 1, r, m));

    let metric: Vec<_> = if let Some(exclusion_list) = exclusion_list {
        let excluded = parse_exclusion_list(&exclusion_list)?;
        iterator
            .filter(|(_, r, _)| excluded.contains(&r.id))
            .collect()
    } else {
        iterator.collect()
    };

    println!("ranking,nickname,fingerprint,ip,bandwidth,metric");
    for (ranking, relay, metric) in metric {
        println!(
            "{},{},{},{},{},{}",
            ranking,
            relay.nickname,
            relay.fingerprint(),
            relay.ip,
            relay.consensus_bandwith,
            metric,
        );
    }

    Ok(())
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
        } => {
            let metric = network_metric::compute(&consensus, &mapping_file, &as_path_file)?;
            rank(metric, exclusion_list)?;
        }
        Command::RelayMetric { exclusion_list } => {
            let metric = relay_metric::compute(&consensus);
            rank(metric, exclusion_list)?;
        }
    }

    Ok(())
}
