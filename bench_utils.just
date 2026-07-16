_cargo_release_path := "./target/release"

@hyperfine bin cores warmup runs release_path=_cargo_release_path:
    cargo build --release
    taskset -c {{cores}} hyperfine --warmup {{warmup}} --runs {{runs}} "{{release_path}}/{{bin}}"

@flamegraph bin release_path=_cargo_release_path:
    cargo build --profile bench
    @sudo perf record -g -- {{release_path}}/{{bin}} >> /dev/null
    @sudo perf script | inferno-collapse-perf | inferno-flamegraph > "flamegraph_{{bin}}.svg"

clean:
    cargo clean
    rm -f perf.data perf.data.old flamegraph_*.svg
