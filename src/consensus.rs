use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV6},
    str::FromStr,
};

use anyhow::{Context, Ok, Result, bail};
use base64::prelude::*;
use bitflags::bitflags;
use derive_more::From;

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

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, From)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Relay {
    nickname: String,
    id: RelayId,
    digest: Digest,
    // publication
    ip: Ipv4Addr,
    orport: Port,
    dirport: Port,
    ipv6: Option<SocketAddrV6>,
    flags: RelayFlags,
    verison: Version,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Consensus {
    relays: Vec<Relay>,
    bandwidth_weights: HashMap<String, u32>,
}

impl FromStr for Consensus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = parse_preamble(s)?;
        let s = parse_authorities(s)?;
        let (relays, s) = parse_relays(s)?;
        let bandwidth_weights = parse_footer(s)?;

        Ok(Self {
            relays,
            bandwidth_weights,
        })
    }
}

fn parse_preamble(s: &str) -> Result<&str> {
    let mut s = s;
    loop {
        let (start, rest) = s.split_once(" ").context("unable to parse")?;
        s = match start {
            "dir-source" => return Ok(s),
            _ => {
                let (_, rest) = rest.split_once("\n").context("unable to parse")?;
                rest
            }
        };
    }
}

fn parse_authorities(s: &str) -> Result<&str> {
    let mut s = s;
    loop {
        let (start, rest) = s.split_once(" ").context("unable to parse")?;
        s = match start {
            "r" => return Ok(s),
            _ => {
                let (_, rest) = rest.split_once("\n").context("unable to parse")?;
                rest
            }
        };
    }
}

fn parse_relays(s: &str) -> Result<(Vec<Relay>, &str)> {
    let mut relays = Vec::new();
    let mut s = s;
    loop {
        let (line, _) = s.split_once("\n").context("unable to split line")?;
        let (start, content) = line.split_once(" ").unwrap_or((line, ""));
        s = match start {
            "r" => {
                let (relay, rest) = parse_relay(s)?;
                relays.push(relay);
                rest
            }
            "directory-footer" => return Ok((relays, s)),
            _ => {
                let (_, rest) = content.split_once("\n").context("unable to split line")?;
                rest
            }
        };
    }
}

fn parse_id(token: &str) -> Result<[u8; 20]> {
    anyhow::ensure!(token.len() == 27);
    let mut id = [0u8; 20];
    BASE64_STANDARD_NO_PAD.decode_slice(token, &mut id)?;
    Ok(id)
}

fn parse_relay(s: &str) -> Result<(Relay, &str)> {
    let mut nickname: Option<String> = None;
    let mut id: Option<RelayId> = None;
    let mut digest: Option<Digest> = None;
    let mut ip: Option<Ipv4Addr> = None;
    let mut oport: Option<Port> = None;
    let mut dirport: Option<Port> = None;
    let mut ipv6: Option<SocketAddrV6> = None;
    let mut flags: Option<RelayFlags> = None;
    let mut version: Option<Version> = None;

    let mut s = s;
    loop {
        let (line, rest) = s.split_once("\n").context("unable to split line")?;
        let (start, content) = line
            .split_once(" ")
            .context("unable to split line content")?;
        match start {
            "r" => {
                let mut parts = content.split(" ");
                nickname = Some(
                    parts
                        .next()
                        .context("unable to parse nickname")?
                        .to_string(),
                );
                id = Some(parts.next().context("unable to parse id")?.parse()?);
                digest = Some(parts.next().context("unable to parse digest")?.parse()?);
                let mut parts = parts.skip(2);
                ip = Some(parts.next().context("unable to parse ip")?.parse()?);
                oport = Some(parts.next().context("unable to parse orport")?.parse()?);
                dirport = Some(parts.next().context("unable to parse dirport")?.parse()?);
            }
            "a" => ipv6 = Some(content.parse()?),
            "s" => flags = Some(parse_flags(content)?),
            "v" => version = Some(content.parse()?),
            "p" => {
                let relay = Relay {
                    nickname: nickname.context("nickname was not provided")?,
                    id: id.context("id was not provided")?,
                    digest: digest.context("digest was not provided")?,
                    ip: ip.context("ip was not provided")?,
                    orport: oport.context("orport was not provided")?,
                    dirport: dirport.context("dirport was not provided")?,
                    ipv6,
                    flags: flags.context("flags were not provided")?,
                    verison: version.context("version was not provided")?,
                };
                return Ok((relay, rest));
            }
            _ => (),
        };
        s = rest;
    }
}

fn parse_flags(s: &str) -> Result<RelayFlags> {
    let mut flags = RelayFlags::empty();
    for token in s.split(" ") {
        flags |= token.parse()?;
    }
    Ok(flags)
}

fn parse_footer(s: &str) -> Result<HashMap<String, u32>> {
    let (_, rest) = s
        .split_once("\n")
        .context("unable to skip directory footer")?;
    let (bdw_wght, _) = rest
        .split_once("\n")
        .context("unable to split bandwidth weights")?;

    Ok(bdw_wght
        .split(" ")
        .filter_map(|w| w.split_once("="))
        .map(|(k, v)| (k.to_string(), v.parse().unwrap_or_default()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read};

    #[test]
    fn footer() {
        let footer = "directory-footer
bandwidth-weights Wbd=1276 Wbe=0 Wbg=4430 Wbm=10000 Wdb=10000 Web=10000 Wed=7447 Wee=10000 Weg=7447 Wem=10000 Wgb=10000 Wgd=1276 Wgg=5570 Wgm=5570 Wmb=10000 Wmd=1276 Wme=0 Wmg=4430 Wmm=10000
directory-signature 0232AF901C31A04EE9848595AF9BB7620D4C5B2E 79ABAEE942F8F0F6E5FEB989D2539B05AB86DD8B
-----BEGIN SIGNATURE-----
Dtx5MNHwIZ/peN77uhDz70LSVHuFcSRZnpvxTnTTI63Sliz8rgAEy7XVn+vVnyQi
TF81h0QR+JyDOiX8UqlKWyV7Q9FdsQ2Q581hiHH7AUOgUh4vKGodr+yijTfdYw5U
9ivFKcLShXfo8m4tSsPpAd0sEpTwL1L9V9Kjb+qR8nb0DNiE+Ntq7AhBFKj6xPPq
U539pXSIgzKgSbAE2pTIEHqLoHVHBStZ5+g6QxY8zp457u0F0kLoS2Qdev69Ayft
UNPylxjvX0nB5XWwlSRhFRjrPHU+l+pi8L7qKiFnrmKQVbb4ADbFQZXDOY7UX6fP
T0YyB5oFq5qcZfS+lNjeiQ==
-----END SIGNATURE-----";
        let _ = parse_footer(footer).unwrap();
    }

    #[test]
    fn relay() {
        let description = "r StarAppsMobley ACg7VWTjBy3N2rMdbvYi3Um/Uk8 1YAPbIMOGGyEAWUQLWGYj6akKK4 2026-04-30 09:36:01 84.234.21.98 9001 0
a [2001:1600:18:100::10c]:9001
s Fast Guard Running Stable V2Dir Valid
v Tor 0.4.9.6
pr Conflux=1 Cons=1-2 Desc=1-4 DirCache=2 FlowCtrl=1-2 HSDir=2 HSIntro=4-5 HSRend=1-2 Link=3-5 LinkAuth=3 Microdesc=1-3 Padding=2 Relay=2-6
w Bandwidth=61000
p reject 1-65535
";
        let _ = parse_relay(description).unwrap();
    }

    #[test]
    fn parse_consensus() {
        let mut file = File::open("test/2026-05-01-00-00-00-consensus").unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let _: Consensus = content.parse().unwrap();
    }
}
