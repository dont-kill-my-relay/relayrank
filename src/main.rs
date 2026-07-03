use anyhow::Result;
use clap::Parser;

mod network_metric;
mod relay_metric;
mod tor_status;

#[derive(Parser)]
enum Args {
    RelayMetric,
    BuildInference,
    NetworkMetric,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args {
        Args::BuildInference => todo!("build inference"),
        Args::NetworkMetric => network_metric::compute(),
        Args::RelayMetric => todo!("relay metric"),
    }
}
