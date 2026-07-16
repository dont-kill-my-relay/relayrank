use std::path::PathBuf;

use anyhow::Result;
use rayon::prelude::*;

use crate::{
    parse_exclusion_list,
    tor_status::{self, Consensus},
};

struct Relay<'a> {
    relay: &'a tor_status::Relay,
    bwg: f64,
    bwe: f64,
}

impl<'a> Relay<'a> {
    fn is_exit(&self) -> bool {
        self.relay.is_exit()
    }

    fn is_guard(&self) -> bool {
        self.relay.is_guard()
    }

    fn reach(&self, other: &Relay) -> bool {
        self.relay.reach(other.relay)
    }
}

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
) -> Vec<Relay<'_>> {
    consensus
        .relays
        .iter()
        .map(|relay| {
            let bwg = relay.bwg(&consensus.bandwidth_weights) / guard_bw_sum;
            let bwe = relay.bwe(&consensus.bandwidth_weights) / exit_bw_sum;
            Relay { relay, bwg, bwe }
        })
        .collect()
}

fn metric_guard(guard: &Relay, relays: &[Relay]) -> f64 {
    let guard_value = guard.bwg;
    let exits_value: f64 = relays
        .iter()
        .filter(|exit| exit.is_exit() && guard.reach(exit))
        .map(|exit| exit.bwe)
        .sum();
    // dbg!((guard_value, exits_value));
    guard_value * exits_value
}

fn metric_exit(exit: &Relay, relays: &[Relay]) -> f64 {
    let exit_value = exit.bwe;
    let guards_value: f64 = relays
        .iter()
        .filter(|guard| guard.is_guard() && exit.reach(guard))
        .map(|guard| guard.bwg)
        .sum();
    // dbg!((exit_value, guards_value));
    exit_value * guards_value
}

fn metric_dual(dual: &Relay, relays: &[Relay]) -> f64 {
    metric_guard(dual, relays) + metric_exit(dual, relays)
}

pub fn compute(consensus: &Consensus, exclusion_list: &Option<PathBuf>) -> Result<()> {
    let (guard_bw_sum, exit_bw_sum) = compute_bandwith_sums(consensus);
    let relays = compute_normalized_bandwidth(consensus, guard_bw_sum, exit_bw_sum);

    let mut metric: Vec<_> = relays
        .par_iter()
        .map(|relay| match (relay.is_guard(), relay.is_exit()) {
            (true, true) => (relay.relay, metric_dual(relay, &relays)),
            (true, false) => (relay.relay, metric_guard(relay, &relays)),
            (false, true) => (relay.relay, metric_exit(relay, &relays)),
            (false, false) => (relay.relay, 0.0),
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
