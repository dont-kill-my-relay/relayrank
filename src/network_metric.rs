use std::path::Path;

use anyhow::{Ok, Result};
use chrono::{TimeZone, Utc};

use crate::tor_status::Consensus;

pub fn compute() -> Result<()> {
    let datetime = Utc.with_ymd_and_hms(2023, 5, 30, 17, 0, 0).unwrap();
    let cache_dir = Path::new("./cache");
    let consensus = Consensus::new(cache_dir, datetime)?;

    for relay in consensus.relays.iter() {
        println!("{}: {}", relay.nickname, relay.bandwidth.average);
    }

    Ok(())
}
