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

The checked-in 2026-08-31 Rust baseline remains the original comparable v1 result.
The clean-restart 2026-09-01 v2 baseline adds the paired-rollout contract without
overwriting that historical evidence.

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

## Clean-restart release measurements (2026-09-01)

The 20-sample single-thread v2 run recorded 472,673.560 transitions/second,
58,879.355 search nodes/second, and 4.059 fully replayed and re-verified
games/second. Median and p95 throughput were 483,329.604 and 499,886.903
transitions/second, 58,951.288 and 60,210.738 search nodes/second, and 4.154 and
4.287 replayed games/second. The lightweight paired workload measured median and
p95 speculative throughput of 1,907.378 and 1,952.972 games/second, estimating
114,442 speculative games in 60 seconds on one thread.

Five 256-game authoritative batches at eight workers measured 18.936 replayed
games/second median and 19.350 peak. Three concurrent eight-worker processes on
the 24-logical-CPU host measured 40.852 games/second median and 41.035 peak over
three 384-game samples: about 2,451 authoritative games in 60 seconds. A separate
eight-worker run observed a 41,181,184-byte peak working set. GPU execution was
not used; the authoritative branch-heavy state machine has no measured GPU-suited
kernel.
