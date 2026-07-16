use anyhow::Result;
use hex::FromHex;
use std::{fs::File, io::Read, path::Path};

use crate::tor_status::RelayId;

pub mod network_metric;
pub mod relay_metric;
pub mod tor_status;

fn parse_exclusion_list(exclusion_list: &Path) -> Result<Vec<RelayId>> {
    let mut exclusion_file = File::open(exclusion_list)?;
    let mut content = String::new();
    exclusion_file.read_to_string(&mut content)?;

    content.lines().map(RelayId::from_hex).collect()
}
