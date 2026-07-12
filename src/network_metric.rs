use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
};

use anyhow::{Context, Ok, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tor_status::{Consensus, Relay, RelayId};

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
struct Mapping {
    client_guard: Vec<RelayId>,
    exit_destination: Vec<RelayId>,
}

type RawASObeservation<'a> = (usize, Option<ASes<'a>>, Option<ASes<'a>>);
type ASes<'a> = HashSet<&'a str>;
type ASObservations<'a> = Vec<ASes<'a>>;
type ASCount<'a> = HashMap<&'a str, u32>;
type ASPropabilty = HashMap<String, f32>;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
struct Inference<'a> {
    guards: HashMap<&'a RelayId, ASObservations<'a>>,
    exits: HashMap<&'a RelayId, ASObservations<'a>>,
}

impl<'a> Inference<'a> {
    fn add_guard_observation(&mut self, id: &'a RelayId, ases: ASes<'a>) {
        let observastions = self.guards.entry(id).or_default();
        observastions.push(ases);
    }

    fn add_exit_observation(&mut self, id: &'a RelayId, ases: ASes<'a>) {
        let observastions = self.exits.entry(id).or_default();
        observastions.push(ases);
    }
}

fn split_observations(
    l: &'_ str,
    guard_sample: usize,
    exit_sample: usize,
) -> Result<(Option<RawASObeservation<'_>>, Option<RawASObeservation<'_>>)> {
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

    let guard_ases = if idx < guard_sample {
        let c2g = extract_asn_path(c2g);
        let g2c = extract_asn_path(g2c);
        Some((idx, c2g, g2c))
    } else {
        None
    };

    let exit_ases = if idx < exit_sample {
        let e2d = extract_asn_path(e2d);
        let d2e = extract_asn_path(d2e);
        Some((idx, e2d, d2e))
    } else {
        None
    };

    Ok((guard_ases, exit_ases))
}

fn extract_asn_path(path: &'_ str) -> Option<ASes<'_>> {
    if path == "None" {
        return None;
    }
    Some(path.split("-").collect())
}

fn fold_observations<'a>(
    mut inference: Inference<'a>,
    l: Result<(Option<RawASObeservation<'a>>, Option<RawASObeservation<'a>>)>,
    mapping: &'a Mapping,
) -> Result<Inference<'a>> {
    let (guard, exit) = l?;

    if let Some((id, c2g, g2c)) = guard {
        let id = &mapping.client_guard[id];
        let ases = c2g
            .unwrap_or_default()
            .union(&g2c.unwrap_or_default())
            .copied()
            .collect();
        inference.add_guard_observation(id, ases);
    }

    if let Some((id, e2d, d2e)) = exit {
        let id = &mapping.exit_destination[id];
        let ases = e2d
            .unwrap_or_default()
            .union(&d2e.unwrap_or_default())
            .copied()
            .collect();
        inference.add_exit_observation(id, ases);
    }

    Ok(inference)
}

fn count_observation(observations: Vec<ASes<'_>>) -> ASCount<'_> {
    observations
        .into_iter()
        .fold(ASCount::new(), |mut count, observation| {
            for asn in observation {
                count.entry(asn).and_modify(|c| *c += 1).or_insert(1);
            }
            count
        })
}

fn asn_proba(observations: HashMap<&'_ RelayId, Vec<ASes>>) -> HashMap<RelayId, ASPropabilty> {
    observations
        .into_iter()
        .map(|(id, count)| {
            let sample_count: f32 = count.len() as f32;
            let proba = count_observation(count)
                .into_iter()
                .map(|(k, v)| (k.to_string(), v as f32 / sample_count))
                .collect();
            (id.clone(), proba)
        })
        .collect()
}

fn extract_pag_pae_from_inference(
    as_path_file: &Path,
    mapping_file: &Path,
) -> Result<(
    HashMap<RelayId, ASPropabilty>,
    HashMap<RelayId, ASPropabilty>,
)> {
    let mapping_file = File::open(mapping_file)?;
    let mapping: Mapping = serde_json::from_reader(mapping_file)?;

    let mut as_path_file = File::open(as_path_file)?;
    let mut as_path = String::new();
    as_path_file.read_to_string(&mut as_path)?;

    let inference = as_path
        .lines()
        .skip(1)
        .map(|l| {
            split_observations(
                l,
                mapping.client_guard.len(),
                mapping.exit_destination.len(),
            )
        })
        .try_fold(Inference::default(), |acc, l| {
            fold_observations(acc, l, &mapping)
        })?;

    Ok((asn_proba(inference.guards), asn_proba(inference.exits)))
}

pub fn compute(
    cache_folder: &Path,
    datetime: DateTime<Utc>,
    mapping_file: &Path,
    as_path_file: &Path,
) -> Result<()> {
    let consensus = Consensus::new(cache_folder, datetime)?;
    let (guards_proba, exits_proba) = extract_pag_pae_from_inference(as_path_file, mapping_file)?;

    let all_ases: HashSet<_> = guards_proba
        .iter()
        .chain(exits_proba.iter())
        .flat_map(|(_, ases)| ases.keys())
        .map(|asn| asn.to_owned())
        .collect();

    let guards: Vec<_> = consensus
        .relays
        .iter()
        .filter(|r| r.is_guard_only())
        .collect();

    let exits: Vec<_> = consensus
        .relays
        .iter()
        .filter(|r| r.is_exit_only())
        .collect();

    let duals: Vec<_> = consensus.relays.iter().filter(|r| r.is_dual()).collect();

    let guards_metric: Vec<(&Relay, f32)> = guards
        .iter()
        .map(|r| (*r, guard_metric(r, &consensus.bandwidth_weights)))
        .collect();
    let exits_metric: Vec<(&Relay, f32)> = exits
        .iter()
        .map(|r| (*r, exit_metric(r, &consensus.bandwidth_weights)))
        .collect();
    let duals_metric: Vec<(&Relay, f32)> = duals
        .iter()
        .map(|r| (*r, dual_metric(r, &consensus.bandwidth_weights)))
        .collect();

    Ok(())
}

fn guard_metric(guard: &Relay, bandwidth_weights: &HashMap<String, u32>) -> f32 {
    let bandwidth_weight = *(bandwidth_weights
        .get("Wgg")
        .expect("Wgg is not in the bandwidth weigths"));
    let guard_bandwidth = (guard.bandwidth.observed as f32) / 1000.0 * bandwidth_weight as f32;

    todo!()
}

fn exit_metric(exit: &Relay, bandwidth_weights: &HashMap<String, u32>) -> f32 {
    todo!()
}

fn dual_metric(dual: &Relay, bandwidth_weights: &HashMap<String, u32>) -> f32 {
    todo!()
}
