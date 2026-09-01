# Engine benchmarks

Run the Rust engine benchmark only in release mode:

```powershell
cargo run --release --locked -p sorcery-engine --bin engine-benchmark > benchmarks/rust-engine-current.json
```

`BENCHMARK_SAMPLES`, `BENCHMARK_GAMES_PER_SAMPLE`,
`BENCHMARK_PAIRED_ROLLOUTS_PER_SAMPLE`, and `BENCHMARK_SEARCH_HORIZON`
override the defaults `5`, `1`, `1`, and `2`.

The compact transition workload enumerates engine-issued actions and applies
the checked-in parity path directly to `Game`, without receipts, state hashes,
or replay serialization. The replay workload runs the same fixture actions
through `Session` and verifies a second authoritative replay. Search clones one
in-memory root per legal action and follows the first canonically ordered action
for the configured horizon. The paired-rollout workload repeats one deterministic
two-seat speculative pair in memory, after replay-verifying both orientations once,
and reports its single-thread 60-second capacity estimate.

For a bounded estimate before the clean-reboot benchmark, run without redirecting
or replacing a checked-in baseline:

```powershell
$env:BENCHMARK_SAMPLES = '5'
$env:BENCHMARK_GAMES_PER_SAMPLE = '1'
$env:BENCHMARK_PAIRED_ROLLOUTS_PER_SAMPLE = '1'
$env:BENCHMARK_SEARCH_HORIZON = '2'
cargo run --release --locked -p sorcery-engine --bin engine-benchmark
```

This estimate is single-threaded, repeats one paired seed, and does not claim
all-core or GPU capacity. Save a new baseline only after the clean-reboot release
run uses the final workload and records the environment.

Output conforms to [rust-engine-benchmark.schema.json](rust-engine-benchmark.schema.json).
`peakRssBytes` is `null` when safe stdlib-only peak RSS measurement is not
available; Linux reports `/proc/self/status` `VmHWM`.

The checked-in 2026-08-31 Rust baseline is the original comparable v1 result and
therefore has no paired-rollout section. It remains historical evidence until a
clean-reboot v2 baseline replaces it; short estimates must not overwrite it.

## Cutover baselines (2026-08-31)

| Release workload | TypeScript | Rust | Ratio |
| --- | ---: | ---: | ---: |
| Transitions/second | 362.222 | 600,051.263 | 1,656.6x |
| Search nodes/second (horizon 2) | 207.255 | 52,806.565 | 254.8x |
| Fully replayed and re-verified games/second | 0.706 | 3.411 | 4.83x |

The JSON baselines retain aggregate, median, and p95 measurements. Rust's
20-sample comparable run exceeds the mature transition target and the initial
3x fully replayed-game target. Windows peak RSS is unavailable in the
stdlib-only harness, so a final Linux release run must supply that measurement
before cutover.
