use std::str::FromStr;

use super::consensus::Digest;

use anyhow::{Context, Ok, bail};
use derive_more::From;
use hex::FromHex;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From)]
pub struct Uptime(u32);

impl FromStr for Uptime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ut: u32 = s.parse()?;
        Ok(ut.into())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Bandwidth {
    average: u32,
    burst: u32,
    observed: u32,
}

impl FromStr for Bandwidth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(" ");
        let (Some(average), Some(burst), Some(observed)) =
            (parts.next(), parts.next(), parts.next())
        else {
            bail!("unable to parse bandwidth")
        };

        Ok(Self {
            average: average.parse()?,
            burst: burst.parse()?,
            observed: observed.parse()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Descriptor {
    uptime: Uptime,
    bandwidth: Bandwidth,
    family: Option<Vec<Digest>>,
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
            family,
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
        let desciptor = include_str!("../test/0a0a2ea40e931a164b2d47af709371c4e9a4bd26");
        desciptor.parse::<Descriptor>().unwrap();
    }
}
