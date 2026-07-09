use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
};

use anyhow::{Context, Ok, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tor_status::{Consensus, Digest, RelayId};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Mapping {
    client_guard: Vec<RelayId>,
    exit_destination: Vec<RelayId>,
}

type ASObservation = HashMap<RelayId, Vec<HashSet<String>>>;
type ASCount = HashMap<RelayId, HashMap<String, u32>>;
type ASPropabilty = HashMap<RelayId, HashMap<String, f32>>;

fn extract_as_path(path: &str) -> Option<HashSet<String>> {
    if path == "None" {
        return None;
    }

    Some(path.split("-").map(|asn| asn.to_string()).collect())
}

fn count_observation(observations: Vec<HashSet<String>>) -> HashMap<String, u32> {
    observations.into_iter().fold(HashMap::new(), |mut acc, o| {
        for asn in o {
            acc.entry(asn).and_modify(|c| *c += 1).or_insert(1);
        }
        acc
    })
}

fn asn_proba(observations: ASObservation) -> ASPropabilty {
    observations
        .into_iter()
        .map(|(id, v)| {
            let sample_count: f32 = v.len() as f32;
            let proba = count_observation(v)
                .into_iter()
                .map(|(k, v)| (k, v as f32 / sample_count))
                .collect();
            (id, proba)
        })
        .collect()
}

fn extract_pag_pae_from_inference(
    as_path_file: &Path,
    mapping_file: &Path,
) -> Result<(ASPropabilty, ASPropabilty)> {
    let mapping_file = File::open(mapping_file)?;
    let mapping: Mapping = serde_json::from_reader(mapping_file)?;

    let mut as_path_file = File::open(as_path_file)?;
    let mut as_path = String::new();
    as_path_file.read_to_string(&mut as_path)?;

    let (guards_observations, exit_obesarvations) = as_path
        .lines()
        .skip(1)
        .map(|l| {
            let mut parts = l.split(" ");
            let (Some(_), Some(idx), Some(c2g), Some(g2c), Some(e2d), Some(d2e)) = (
                parts.next(),
                parts.next(),
                parts.next(),
                parts.next(),
                parts.next(),
                parts.next(),
            ) else {
                bail!(format!("unable to parse inference line: {l:?}"))
            };
            let idx: usize = idx.parse().with_context(|| format!("invalid id {idx:?}"))?;

            let guard_ases: Option<(RelayId, HashSet<_>)> = if idx < mapping.client_guard.len() {
                let c2g = extract_as_path(c2g).unwrap_or_default();
                let g2c = extract_as_path(g2c).unwrap_or_default();
                let id: RelayId = mapping.client_guard[idx].clone();

                Some((id, c2g.union(&g2c).cloned().collect()))
            } else {
                None
            };

            let exit_ases: Option<(RelayId, HashSet<_>)> = if idx < mapping.exit_destination.len() {
                let e2d = extract_as_path(e2d).unwrap_or_default();
                let d2e = extract_as_path(d2e).unwrap_or_default();
                let id: RelayId = mapping.exit_destination[idx].clone();

                Some((id, e2d.union(&d2e).cloned().collect()))
            } else {
                None
            };

            Ok((guard_ases, exit_ases))
        })
        .try_fold(
            (ASObservation::new(), ASObservation::new()),
            |(mut guards, mut exits), l| {
                let (guard_ases, exit_ases) = l?;

                if let Some((id, ases)) = guard_ases {
                    let observations = guards.entry(id).or_default();
                    observations.push(ases);
                }

                if let Some((id, ases)) = exit_ases {
                    let observations = exits.entry(id).or_default();
                    observations.push(ases);
                }

                Ok((guards, exits))
            },
        )?;

    Ok((
        asn_proba(guards_observations),
        asn_proba(exit_obesarvations),
    ))
}

pub fn compute(
    cache_folder: &Path,
    datetime: DateTime<Utc>,
    mapping_file: &Path,
    as_path_file: &Path,
) -> Result<()> {
    let consensus = Consensus::new(cache_folder, datetime)?;
    dbg!("ici");
    let proba = extract_pag_pae_from_inference(as_path_file, mapping_file)?;
    dbg!(proba);

    Ok(())
}
