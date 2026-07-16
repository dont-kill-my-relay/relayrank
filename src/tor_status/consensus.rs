use crate::tor_status::BandwithWeights;

use super::{Digest, Port, RelayFlags, RelayId, Version};
use std::{
    net::{Ipv4Addr, SocketAddrV6},
    str::FromStr,
};

use anyhow::{Context, Result, bail};

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
    pub bandwidth: u32,
    pub verison: Version,
}

impl Relay {
    fn parse_relay(s: &str) -> Result<(Self, &str)> {
        let mut nickname: Option<String> = None;
        let mut id: Option<RelayId> = None;
        let mut digest: Option<Digest> = None;
        let mut ip: Option<Ipv4Addr> = None;
        let mut oport: Option<Port> = None;
        let mut dirport: Option<Port> = None;
        let mut ipv6: Option<SocketAddrV6> = None;
        let mut flags: Option<RelayFlags> = None;
        let mut version: Option<Version> = None;
        let mut bandwidth: Option<u32> = None;

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
                "s" => flags = Some(Self::parse_flags(content)?),
                "v" => version = Some(content.parse()?),
                "w" => {
                    let (content, _) = content.split_once(" ").unwrap_or((content, ""));
                    let Some((_, bdw)) = content.split_once("=") else {
                        bail!("unable to parse bandwith")
                    };
                    bandwidth =
                        Some(bdw.parse().with_context(|| {
                            format!("unable to parse bandwidth: {:?}", content)
                        })?);
                }
                "p" => {
                    let relay = Self {
                        nickname: nickname.context("nickname was not provided")?,
                        id: id.context("id was not provided")?,
                        digest: digest.context("digest was not provided")?,
                        ip: ip.context("ip was not provided")?,
                        orport: oport.context("orport was not provided")?,
                        dirport: dirport.context("dirport was not provided")?,
                        ipv6,
                        flags: flags.context("flags were not provided")?,
                        verison: version.context("version was not provided")?,
                        bandwidth: bandwidth.context("bandwidth was not provided")?,
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
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Consensus {
    pub relays: Vec<Relay>,
    pub bandwidth_weights: BandwithWeights,
}

impl Consensus {
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
                    let (relay, rest) = Relay::parse_relay(s)?;

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

    fn parse_footer(s: &str) -> Result<BandwithWeights> {
        let (_, rest) = s
            .split_once("\n")
            .context("unable to skip directory footer")?;
        let (bdw_wght, _) = rest
            .split_once("\n")
            .context("unable to split bandwidth weights")?;

        bdw_wght
            .split(" ")
            .skip(1)
            .map(|w| {
                let (k, v) = w
                    .split_once("=")
                    .with_context(|| format!("unable to parse bandwith weights {w:?}"))?;
                Ok((k.to_string(), v.parse()?))
            })
            .collect()
    }
}

impl FromStr for Consensus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = Self::parse_preamble(s)?;
        let s = Self::parse_authorities(s)?;
        let (relays, s) = Self::parse_relays(s)?;
        let bandwidth_weights = Self::parse_footer(s)?;

        Ok(Self {
            relays,
            bandwidth_weights,
        })
    }
}

#[cfg(feature = "bench")]
pub mod bench_utils {
    pub use super::Consensus;
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let _ = Consensus::parse_footer(footer).unwrap();
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
        let _ = Relay::parse_relay(description).unwrap();
    }

    #[test]
    fn parse_consensus() {
        let content = include_str!("../../test/2026-05-01-00-00-00-consensus");
        let _: Consensus = content.parse().unwrap();
    }
}
