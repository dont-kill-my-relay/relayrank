mod bench_utils

alias clean:=bench_utils::clean
alias c:=bench_utils::clean
alias b:=bench

bin := "RelayRank"
relay_metric := f"{{bin}} 2023-05-15T12:00:00Z relay-metric"

[arg("cores", short)]
bench cores="0": (bench_utils::hyperfine relay_metric cores "20" "50")

flamegraph: (bench_utils::flamegraph relay_metric)
