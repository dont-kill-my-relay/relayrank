use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok, Result, bail};
use chrono::{DateTime, Utc};
use hex::FromHex;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::tor_status::{BandwithWeights, Consensus, Relay, RelayId};

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
struct Mapping {
    client_guard: Vec<RelayId>,
    exit_destination: Vec<RelayId>,
}

type Probability<EVENT> = HashMap<EVENT, f64>;
type ConditionnalProbability<EVENT, CODITION> = HashMap<CODITION, Probability<EVENT>>;

type RawASObeservation<'a> = (usize, Option<ASes<'a>>, Option<ASes<'a>>);
type ASes<'a> = HashSet<&'a str>;
type ASObservations<'a> = Vec<ASes<'a>>;
type ASCount<'a> = HashMap<&'a str, u32>;

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

fn parse_exclusion_list(exclusion_list: &Path) -> Result<Vec<RelayId>> {
    let mut exclusion_file = File::open(exclusion_list)?;
    let mut content = String::new();
    exclusion_file.read_to_string(&mut content)?;

    content.lines().map(RelayId::from_hex).collect()
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

fn asn_proba(
    observations: HashMap<&'_ RelayId, Vec<ASes>>,
) -> ConditionnalProbability<String, RelayId> {
    observations
        .into_iter()
        .map(|(id, count)| {
            let sample_count: f64 = count.len() as f64;
            let proba = count_observation(count)
                .into_iter()
                .map(|(k, v)| (k.to_string(), v as f64 / sample_count))
                .collect();
            (id.clone(), proba)
        })
        .collect()
}

fn extract_pag_pae_from_inference(
    as_path_file: &Path,
    mapping_file: &Path,
) -> Result<(
    ConditionnalProbability<String, RelayId>,
    ConditionnalProbability<String, RelayId>,
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

pub fn peg<'a>(
    relays: &'a [Relay],
    bandwidth_weights: &BandwithWeights,
) -> ConditionnalProbability<&'a RelayId, &'a RelayId> {
    relays
        .iter()
        .filter(|guard| guard.is_guard())
        .map(|guard| {
            let sum: f64 = relays
                .iter()
                .filter(|e| e.is_exit() && guard.reach(e))
                .map(|e| e.bwe(bandwidth_weights))
                .sum();
            let exits: Probability<&RelayId> = relays
                .iter()
                .filter(|e| e.is_exit() && guard.reach(e))
                .map(|e| {
                    let proba = e.bwe(bandwidth_weights) / sum;
                    (&e.id, proba)
                })
                .collect();
            (&guard.id, exits)
        })
        .collect()
}

pub fn pg<'a>(
    relays: &'a [Relay],
    bandwidth_weights: &BandwithWeights,
) -> Probability<&'a RelayId> {
    let sum: f64 = relays
        .iter()
        .filter(|r| r.is_guard())
        .map(|g| g.bwg(bandwidth_weights))
        .sum();
    relays
        .iter()
        .filter(|r| r.is_guard())
        .map(|e| (&e.id, e.bwg(bandwidth_weights) / sum))
        .collect()
}

pub fn pe<'a>(
    relays: &'a [Relay],
    pg: &Probability<&RelayId>,
    peg: &ConditionnalProbability<&RelayId, &RelayId>,
) -> Probability<&'a RelayId> {
    relays
        .iter()
        .filter(|exit| exit.is_exit())
        .map(|exit| {
            let proba: f64 = relays
                .iter()
                .filter(|guard| guard.is_guard() && exit.reach(guard)) //TODO: remove reach ??
                .map(|guard| {
                    let pg = pg.get(&guard.id).expect("guard should be in pg");
                    let peg = peg
                        .get(&guard.id)
                        .expect("guard should be in peg")
                        .get(&exit.id)
                        .expect("exit should be in peg");
                    peg * pg
                })
                .sum();
            (&exit.id, proba)
        })
        .collect()
}

pub fn pge<'a>(
    relays: &'a [Relay],
    pg: &Probability<&RelayId>,
    pe: &Probability<&RelayId>,
    peg: &ConditionnalProbability<&RelayId, &RelayId>,
) -> ConditionnalProbability<&'a RelayId, &'a RelayId> {
    relays
        .iter()
        .filter(|exit| exit.is_exit())
        .map(|exit| {
            let pe = pe.get(&exit.id).expect("exit should be in pe");
            let proba: Probability<&RelayId> = relays
                .iter()
                .filter(|guard| guard.is_guard() && guard.reach(exit))
                .map(|guard| {
                    let pg = pg.get(&guard.id).expect("guard should be in pg");
                    let peg = peg
                        .get(&guard.id)
                        .expect("guard should be in peg")
                        .get(&exit.id)
                        .expect("exit should be in peg");
                    (&guard.id, pg * peg / pe)
                })
                .collect();
            (&exit.id, proba)
        })
        .collect()
}

fn page(
    asn: &str,
    exit: &Relay,
    relays: &[Relay],
    pag: &ConditionnalProbability<String, RelayId>,
    pge: &ConditionnalProbability<&RelayId, &RelayId>,
) -> f64 {
    debug_assert!(exit.is_exit());
    relays
        .iter()
        .filter(|guard| guard.is_guard() && exit.reach(guard))
        .filter_map(|guard| {
            let pag = pag
                .get(&guard.id)
                .expect("guard should be in pag")
             position   .get(asn)?;
            if *pag == 0.0 {
                return Some(0.0);
            }
            let pge = pge
                .get(&exit.id)
                .expect("exit should be in pge")
                .get(&guard.id)
                .expect("guard should be in pge");
            debug_assert!(!pag.is_nan(), "{pag}");
            Some(pag * pge)
        })
        .sum()
}

fn paeg(
    asn: &str,
    guard: &Relay,
    relays: &[Relay],
    pae: &ConditionnalProbability<String, RelayId>,
    peg: &ConditionnalProbability<&RelayId, &RelayId>,
) -> f64 {
    debug_assert!(guard.is_guard());
    relays
        .iter()
        .filter(|exit| exit.is_exit() && guard.reach(exit))
        .filter_map(|exit| {
            let pae = pae.get(&exit.id).expect("exit should be in pae").get(asn)?;
            if *pae == 0.0 {
                return Some(0.0);
            }
            let peg = peg
                .get(&guard.id)
                .expect("guard should be in peg")
                .get(&exit.id)
                .expect("exit should be in peg");
            debug_assert!(!pae.is_nan(), "{pae}");
            Some(pae * peg)
        })
        .sum()
}

fn guard_metric(
    guard: &Relay,
    ases: &HashSet<String>,
    relays: &[Relay],
    pag: &ConditionnalProbability<String, RelayId>,
    pae: &ConditionnalProbability<String, RelayId>,
    peg: &ConditionnalProbability<&RelayId, &RelayId>,
    bandwidth_weights: &BandwithWeights,
) -> f64 {
    debug_assert!(guard.is_guard());
    let bw = guard.bwg(bandwidth_weights);
    let product: f64 = ases
        .iter()
        .filter_map(|asn| {
            let pag = pag
                .get(&guard.id)
                .expect("guard should be in pag")
                .get(asn)?;
            let paeg = paeg(asn, guard, relays, pae, peg);
            Some(1.0 - (pag * paeg))
        })
        .product();
    debug_assert!(
        !(bw * product).is_nan(),
        "guard_metric = {bw} * {product} = {}",
        bw * product
    );
    bw * product
}

fn exit_metric(
    exit: &Relay,
    ases: &HashSet<String>,
    relays: &[Relay],
    pae: &ConditionnalProbability<String, RelayId>,
    pag: &ConditionnalProbability<String, RelayId>,
    pge: &ConditionnalProbability<&RelayId, &RelayId>,
    bandwidth_weights: &BandwithWeights,
) -> f64 {
    debug_assert!(exit.is_exit());
    let bw = exit.bwe(bandwidth_weights);
    let product: f64 = ases
        .iter()
        .filter_map(|asn| {
            let pae = pae.get(&exit.id).expect("exit should be in pae").get(asn)?;
            let page = page(asn, exit, relays, pag, pge);
            Some(1.0 - (pae * page))
        })
        .product();
    bw * product
}

fn dual_metric(
    dual: &Relay,
    ases: &HashSet<String>,
    relays: &[Relay],
    pae: &ConditionnalProbability<String, RelayId>,
    pag: &ConditionnalProbability<String, RelayId>,
    pge: &ConditionnalProbability<&RelayId, &RelayId>,
    peg: &ConditionnalProbability<&RelayId, &RelayId>,
    bandwidth_weights: &BandwithWeights,
) -> f64 {
    debug_assert!(dual.is_dual());
    let as_guard = guard_metric(dual, ases, relays, pag, pae, peg, bandwidth_weights);
    let as_exit = exit_metric(dual, ases, relays, pae, pag, pge, bandwidth_weights);
    as_guard + as_exit
}

pub fn compute(
    cache_folder: &Path,
    datetime: DateTime<Utc>,
    mapping_file: &Path,
    as_path_file: &Path,
    exclusion_list: &Option<PathBuf>,
) -> Result<()> {
    let consensus = Consensus::new(cache_folder, datetime)?;
    let (pag, pae) = extract_pag_pae_from_inference(as_path_file, mapping_file)?;

    let pg = pg(&consensus.relays, &consensus.bandwidth_weights);
    let peg = peg(&consensus.relays, &consensus.bandwidth_weights);
    let pe = pe(&consensus.relays, &pg, &peg);
    let pge = pge(&consensus.relays, &pg, &pe, &peg);

    let ases: HashSet<_> = pag
        .iter()
        .chain(pae.iter())
        .flat_map(|(_, ases)| ases.keys())
        .map(|asn| asn.to_owned())
        .collect();

    let mut metric: Vec<_> = consensus
        .relays
        .par_iter()
        .filter_map(|relay| {
            if relay.is_dual() {
                Some((
                    relay,
                    dual_metric(
                        relay,
                        &ases,
                        &consensus.relays,
                        &pae,
                        &pag,
                        &pge,
                        &peg,
                        &consensus.bandwidth_weights,
                    ),
                ))
            } else if relay.is_guard() {
                Some((
                    relay,
                    guard_metric(
                        relay,
                        &ases,
                        &consensus.relays,
                        &pag,
                        &pae,
                        &peg,
                        &consensus.bandwidth_weights,
                    ),
                ))
            } else if relay.is_exit() {
                Some((
                    relay,
                    exit_metric(
                        relay,
                        &ases,
                        &consensus.relays,
                        &pae,
                        &pag,
                        &pge,
                        &consensus.bandwidth_weights,
                    ),
                ))
            } else {
                None
            }
        })
        .map(|(r, m)| (r, m))
        .collect();

    metric.sort_by(|(_, m), (_, o)| m.total_cmp(o));
    metric.reverse();
    let iterator = metric.iter().enumerate().map(|(i, (r, m))| (i + 1, r, m));

    let metric: Vec<_> = if let Some(exclusion_list) = exclusion_list {
        let excluded = parse_exclusion_list(exclusion_list)?;
        iterator
            .filter(|(_, r, _)| excluded.contains(&r.id))
            .collect()
    } else {
        iterator.collect()
    };

    for (ranking, relay, metric) in metric {
        println!(
            "{},{},{},{}",
            ranking,
            relay.nickname,
            relay.fingerprint(),
            metric,
        );
    }

    Ok(())
}
