bin := "RelayRank"
release_path := "./target/release"
relay_metric := f"{{bin}} 2023-05-15T12:00:00Z relay-metric"

bench: (_bench relay_metric)

@_bench bin:
    cargo build --quiet --release
    hyperfine --warmup 20  --runs 50 "{{release_path}}/{{bin}}"

flamegraph: famegraph_relay-metric

@_famegraph bin:
    cargo build --quiet --profile bench
    perf record -g -- {{release_path}}/{{bin}} >> /dev/null
    perf script | inferno-collapse-perf | inferno-flamegraph > "flamegraph_{{bin}}.svg"

famegraph_relay-metric: (_famegraph relay_metric)

clean:
    cargo clean
    rm -f perf.data perf.data.old flamegraph_*.svg
