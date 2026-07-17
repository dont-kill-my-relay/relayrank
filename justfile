bin := "relay-rank"
relay_metric_args := "2023-05-15T12:00:00Z relay-metric"
network_metric_args := "2022-10-05T12:00:00Z network-metric mapping_infer-2022-10-05T12.json 2022-10-07-asinfer-metric.txt"
_cargo_release_path := "./target/release"

alias c:=clean
alias b:=bench

[arg("cores", short)]
bench cores="0":
    taskset -c {{cores}} cargo criterion --features bench --message-format=json | jq -SMc "select(has(\"id\"))" > benches.json

_hyperfine cores warmup runs bin args="" release_path=_cargo_release_path:
    cargo build --release
    taskset -c {{cores}} hyperfine --warmup {{warmup}} --runs {{runs}} "{{release_path}}/{{bin}} {{args}}"

[env("CARGO_PROFILE_RELEASE_DEBUG", "true")]
[env("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS", "-Clink-arg=-Wl,--no-rosegment")]
_flamegraph params="":
    cargo flamegraph -- {{params}}

[arg("cores", short)]
hyperfine_relay_metric cores="0": (_hyperfine cores "20" "50" bin relay_metric_args)
[arg("cores", short)]
hyperfine_network_metric cores="0": (_hyperfine cores "1" "5" bin network_metric_args)

flamegraph_relay_metric: (_flamegraph relay_metric_args)
flamegraph_network_metric: (_flamegraph network_metric_args)

clean:
    cargo clean
    rm -f perf.data perf.data.old flamegraph.svg benches.json
