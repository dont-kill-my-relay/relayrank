use std::{fs::File, path::Path};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tor_status::{Consensus, RelayId};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Mapping {
    client_guard: Vec<RelayId>,
    exit_destination: Vec<RelayId>,
}

pub fn compute(cache_folder: &Path, datetime: DateTime<Utc>, mapping_file: &Path) -> Result<()> {
    let consensus = Consensus::new(cache_folder, datetime)?;

    for relay in consensus.relays() {
        println!("{}: {}", relay.nickname, relay.bandwidth.average);
    }

    let mapping_file = File::open(mapping_file)?;
    let mapping: Mapping = serde_json::from_reader(mapping_file)?;

    Ok(())
}
