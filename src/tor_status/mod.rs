use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    net::{Ipv4Addr, SocketAddrV6},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use base64::prelude::*;
use bitflags::bitflags;
use chrono::{DateTime, Datelike, Timelike, Utc};
use derive_more::From;
use hex::{FromHex, ToHex};

use descriptor::Descriptor;

mod consensus;
mod descriptor;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From)]
pub struct RelayId([u8; 20]);

impl FromStr for RelayId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(parse_id(s)?.into())
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From)]
pub struct Digest([u8; 20]);

impl FromStr for Digest {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(parse_id(s)?.into())
    }
}

impl FromHex for Digest {
    type Error = anyhow::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::prelude::v1::Result<Self, Self::Error> {
        let digest = <[u8; 20]>::from_hex(hex)?;
        Ok(digest.into())
    }
}

impl ToHex for Digest {
    fn encode_hex<T: std::iter::FromIterator<char>>(&self) -> T {
        self.0.encode_hex()
    }

    fn encode_hex_upper<T: std::iter::FromIterator<char>>(&self) -> T {
        self.0.encode_hex_upper()
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From, Copy)]
pub struct Port(u16);

impl FromStr for Port {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let n: u16 = s.parse()?;
        Ok(n.into())
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct RelayFlags: u16 {
        const Authority = 0x0001;
        const BadExit = 0x0002;
        const Exit = 0x0004;
        const Fast = 0x0008;
        const Guard = 0x0010;
        const HSDir = 0x0020;
        const MiddleOnly = 0x0040;
        const NoEdConsensus = 0x0080;
        const Stable = 0x0100;
        const StaleDesc = 0x0200;
        const Sybil = 0x0400;
        const Running = 0x0800;
        const Valid = 0x1000;
        const V2Dir = 0x2000;
    }
}

impl FromStr for RelayFlags {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        match s {
            "Authority" => Ok(RelayFlags::Authority),
            "BadExit" => Ok(RelayFlags::BadExit),
            "Exit" => Ok(RelayFlags::Exit),
            "Fast" => Ok(RelayFlags::Fast),
            "Guard" => Ok(RelayFlags::Guard),
            "HSDir" => Ok(RelayFlags::HSDir),
            "MiddleOnly" => Ok(RelayFlags::MiddleOnly),
            "NoEdConsensus" => Ok(RelayFlags::NoEdConsensus),
            "Stable" => Ok(RelayFlags::Stable),
            "StaleDesc" => Ok(RelayFlags::StaleDesc),
            "Sybil" => Ok(RelayFlags::Sybil),
            "Running" => Ok(RelayFlags::Running),
            "Valid" => Ok(RelayFlags::Valid),
            "V2Dir" => Ok(RelayFlags::V2Dir),
            _ => bail!("unknown relay flags!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u8,
    minor: u8,
    micro: u8,
    patch: u8,
    status: Option<String>,
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let (_, version) = s.split_once(" ").context("unable to parse version")?;

        let (numbers, status) = version
            .split_once("-")
            .map(|(n, v)| (n, Some(v.to_owned())))
            .unwrap_or((version, None));

        let mut numbers = numbers.splitn(4, ".");
        let result = match (
            numbers.next(),
            numbers.next(),
            numbers.next(),
            numbers.next(),
        ) {
            (Some(major), Some(minor), Some(micro), Some(patch)) => Self {
                major: major.parse()?,
                minor: minor.parse()?,
                micro: micro.parse()?,
                patch: patch.parse()?,
                status,
            },
            _ => bail!("unable to parse verison"),
        };
        Ok(result)
    }
}

fn parse_id(token: &str) -> Result<[u8; 20]> {
    anyhow::ensure!(token.len() == 27);
    let mut id = [0u8; 20];
    BASE64_STANDARD_NO_PAD.decode_slice(token, &mut id)?;
    Ok(id)
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From, Copy)]
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
    pub average: u32,
    pub burst: u32,
    pub observed: u32,
}

impl FromStr for Bandwidth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(" ");
        let (Some(average), Some(burst), Some(observed)) =
            (parts.next(), parts.next(), parts.next())
        else {
            bail!(format!("unable to parse bandwidth from: {s:?}"))
        };

        Ok(Self {
            average: average.parse()?,
            burst: burst.parse()?,
            observed: observed.parse()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FamilyMember {
    Digest(Digest),
    Nickname(String),
}

impl FromStr for FamilyMember {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        if let Some(d) = s.strip_prefix("$") {
            Ok(Self::Digest(Digest::from_hex(d).with_context(|| {
                format!("unable to parse family member: {s:?}")
            })?))
        } else {
            Ok(Self::Nickname(s.to_string()))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Relay {
    pub nickname: String,
    pub id: RelayId,
    pub digest: Digest,
    // publication
    pub ip: Ipv4Addr,
    pub orport: Port,
    pub dirport: Port,
    pub ipv6: Option<SocketAddrV6>,
    pub flags: RelayFlags,
    pub verison: Version,
    pub uptime: Uptime,
    pub bandwidth: Bandwidth,
    pub family: Vec<FamilyMember>,
}

impl From<(consensus::Relay, descriptor::Descriptor)> for Relay {
    fn from((relay, descriptor): (consensus::Relay, descriptor::Descriptor)) -> Self {
        Relay {
            nickname: relay.nickname,
            id: relay.id,
            digest: relay.digest,
            ip: relay.ip,
            orport: relay.orport,
            dirport: relay.dirport,
            ipv6: relay.ipv6,
            flags: relay.flags,
            verison: relay.verison,
            uptime: descriptor.uptime,
            bandwidth: descriptor.bandwidth,
            family: descriptor.family,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Consensus {
    relays: Vec<Relay>,
    bandwidth_weights: HashMap<String, u32>,
}

impl Consensus {
    pub fn new(cache_folder: &Path, datetime: DateTime<Utc>) -> Result<Self> {
        let filename = format!(
            "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}-consensus",
            datetime.year(),
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
            datetime.second()
        );
        let consensus_path = cache_folder.join("consensuses").join(filename);
        let mut consensus_file = File::open(&consensus_path)
            .with_context(|| format!("unable to open consensus file: {:?}", consensus_path))?;
        let mut content = String::new();

        consensus_file.read_to_string(&mut content)?;

        let consensus: consensus::Consensus = content.parse()?;
        let relays: Result<Vec<Relay>> = consensus
            .relays
            .into_iter()
            .map(|relay| -> Result<Relay> {
                let desc = Descriptor::get_descriptor(cache_folder, &datetime, &relay.digest)?;
                Ok((relay, desc).into())
            })
            .collect();

        Ok(Self {
            relays: relays?,
            bandwidth_weights: consensus.bandwidth_weights,
        })
    }

    pub fn relays(&self) -> &[Relay] {
        &self.relays
    }
}
