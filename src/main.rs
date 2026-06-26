use clap::Parser;

mod consensus;
mod descriptor;
mod network_metric;
mod relay_metric;

#[derive(Parser)]
enum Args {
    RelayMetric,
    BuildInference,
    NetworkMetric,
}

fn main() {
    let args = Args::parse();
    match args {
        Args::BuildInference => todo!("build inference"),
        Args::NetworkMetric => todo!("network metric"),
        Args::RelayMetric => todo!("relay metric"),
    }
}
