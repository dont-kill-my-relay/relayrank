use criterion::{Criterion, criterion_group, criterion_main};
use relay_rank::tor_status::bench_utils::descriptor::Descriptor;

pub fn criterion_benchmark(c: &mut Criterion) {
    let descriptor = include_str!("../test/0a0a2ea40e931a164b2d47af709371c4e9a4bd26");
    c.bench_function("parse_descriptor", |b| {
        b.iter_with_large_drop(|| {
            let descriptor: Descriptor = descriptor.parse().expect("parsing should be correct");
            descriptor
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
