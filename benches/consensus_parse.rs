use criterion::{Criterion, criterion_group, criterion_main};
use relay_rank::tor_status::bench_utils::consensus::Consensus;

pub fn criterion_benchmark(c: &mut Criterion) {
    let consensus = include_str!("../test/2026-05-01-00-00-00-consensus");
    c.bench_function("parse_consensus", |b| {
        b.iter_with_large_drop(|| {
            let consensus: Consensus = consensus.parse().expect("parsing should be correct");
            consensus
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
