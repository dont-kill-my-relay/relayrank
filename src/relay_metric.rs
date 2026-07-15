use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use rayon::prelude::*;

use crate::{
    parse_exclusion_list,
    tor_status::{Consensus, Relay, RelayId},
};

fn compute_bandwith_sums(consensus: &Consensus) -> (f64, f64) {
    consensus
        .relays
        .iter()
        .fold((0.0, 0.0), |(g_sum, e_sum), relay| {
            let g_sum = if relay.is_guard() {
                g_sum + relay.bwg(&consensus.bandwidth_weights)
            } else {
                g_sum
            };
            let e_sum = if relay.is_exit() {
                e_sum + relay.bwe(&consensus.bandwidth_weights)
            } else {
                e_sum
            };
            (g_sum, e_sum)
        })
}

fn compute_normalized_bandwidth(
    consensus: &Consensus,
    guard_bw_sum: f64,
    exit_bw_sum: f64,
) -> (HashMap<&RelayId, f64>, HashMap<&RelayId, f64>) {
    consensus.relays.iter().fold(
        (HashMap::new(), HashMap::new()),
        |(mut guards, mut exits), relay| {
            if relay.is_guard() {
                let bw = relay.bwg(&consensus.bandwidth_weights);
                guards.insert(&relay.id, bw / guard_bw_sum);
            }

            if relay.is_exit() {
                let bw = relay.bwe(&consensus.bandwidth_weights);
                exits.insert(&relay.id, bw / exit_bw_sum);
            }

            (guards, exits)
        },
    )
}

fn metric_guard(
    guard: &Relay,
    relays: &[Relay],
    guard_bw: &HashMap<&RelayId, f64>,
    exit_bw: &HashMap<&RelayId, f64>,
) -> f64 {
    let guard_value = guard_bw
        .get(&guard.id)
        .expect("sguard should be in guard_bw");
    let exits_value: f64 = relays
        .iter()
        .filter(|exit| exit.is_exit() && guard.reach(exit))
        .map(|exit| exit_bw.get(&exit.id).expect("exit shoulb be in exit_bw"))
        .sum();
    // dbg!((guard_value, exits_value));
    guard_value * exits_value
}

fn metric_exit(
    exit: &Relay,
    relays: &[Relay],
    guard_bw: &HashMap<&RelayId, f64>,
    exit_bw: &HashMap<&RelayId, f64>,
) -> f64 {
    let exit_value = exit_bw.get(&exit.id).expect("exit should be in exit_bw");
    let guards_value: f64 = relays
        .iter()
        .filter(|guard| guard.is_guard() && exit.reach(guard))
        .map(|guard| {
            guard_bw
                .get(&guard.id)
                .expect("guard should be in guard_bw")
        })
        .sum();
    // dbg!((exit_value, guards_value));
    exit_value * guards_value
}

fn metric_dual(
    dual: &Relay,
    relays: &[Relay],
    guard_bw: &HashMap<&RelayId, f64>,
    exit_bw: &HashMap<&RelayId, f64>,
) -> f64 {
    metric_guard(dual, relays, guard_bw, exit_bw) + metric_exit(dual, relays, guard_bw, exit_bw)
}

pub fn compute(consensus: &Consensus, exclusion_list: &Option<PathBuf>) -> Result<()> {
    let (guard_bw_sum, exit_bw_sum) = compute_bandwith_sums(consensus);
    let (guard_bw, exit_bw) = compute_normalized_bandwidth(consensus, guard_bw_sum, exit_bw_sum);

    let mut metric: Vec<_> = consensus
        .relays
        .par_iter()
        .map(|relay| match (relay.is_guard(), relay.is_exit()) {
            (true, true) => (
                relay,
                metric_dual(relay, &consensus.relays, &guard_bw, &exit_bw),
            ),
            (true, false) => (
                relay,
                metric_guard(relay, &consensus.relays, &guard_bw, &exit_bw),
            ),
            (false, true) => (
                relay,
                metric_exit(relay, &consensus.relays, &guard_bw, &exit_bw),
            ),
            (false, false) => (relay, 0.0),
        })
        .collect();

    metric.sort_by(|(_, m), (_, o)| m.total_cmp(o));
    metric.reverse();
    let iterator = metric
        .iter()
        .enumerate()
        .map(|(i, (r, m))| (i + 1, r, 1e9 * m));

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
