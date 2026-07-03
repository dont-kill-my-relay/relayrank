use crate::tor_status::consensus;

use super::{Bandwidth, Uptime};
use std::{fs::File, io::Read, path::Path, str::FromStr};

use super::Digest;

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Utc};
use hex::{FromHex, ToHex};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Descriptor {
    pub uptime: Uptime,
    pub bandwidth: Bandwidth,
    pub family: Vec<Digest>,
}

impl Descriptor {
    pub fn get_descriptor(
        cache_folder: &Path,
        datetime: &DateTime<Utc>,
        relay: &consensus::Relay,
    ) -> Result<Self> {
        let descriptor_folder = cache_folder.join("relay_descriptors");
        let month_descriptors = descriptor_folder.join(format!(
            "server-descriptors-{:04}-{:02}",
            datetime.year(),
            datetime.month()
        ));
        let descriptor = month_descriptors.join(relay.digest.encode_hex::<String>());

        let mut descriptor = File::open(descriptor)?;
        let mut content = String::new();

        descriptor.read_to_string(&mut content)?;

        content.parse()
    }
}

impl FromStr for Descriptor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut uptime: Option<Uptime> = None;
        let mut bandwidth: Option<Bandwidth> = None;
        let mut family: Option<Vec<Digest>> = None;

        for line in s.lines() {
            let (start, content) = line.split_once(" ").unwrap_or(("", line));
            match start {
                "uptime" => {
                    uptime = Some(content.parse()?);
                }
                "bandwidth" => {
                    bandwidth = Some(content.parse()?);
                }
                "family" => {
                    family = Some(
                        content
                            .split(" ")
                            .filter_map(|d| Digest::from_hex(&d[1..]).ok())
                            .collect(),
                    )
                }
                _ => (),
            }
        }

        Ok(Self {
            uptime: uptime.context("uptime is not provided")?,
            bandwidth: bandwidth.context("bandwith is not provided")?,
            family: family.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandwith() {
        "4194304 5242880 4655958".parse::<Bandwidth>().unwrap();
    }

    #[test]
    fn descriptor() {
        let desciptor = include_str!("../../test/0a0a2ea40e931a164b2d47af709371c4e9a4bd26");
        desciptor.parse::<Descriptor>().unwrap();
    }
}
