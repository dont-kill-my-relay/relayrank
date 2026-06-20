use clap::Parser;

mod consensus;
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
        Args::BuildInference => println!("build inference"),
        Args::NetworkMetric => println!("network metric"),
        Args::RelayMetric => println!("relay metric"),
    }
}
