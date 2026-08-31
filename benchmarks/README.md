# Engine benchmarks

Run the Rust engine benchmark only in release mode:

```powershell
cargo run --release --locked -p sorcery-engine --bin engine-benchmark > benchmarks/rust-engine-current.json
```

`BENCHMARK_SAMPLES`, `BENCHMARK_GAMES_PER_SAMPLE`, and
`BENCHMARK_SEARCH_HORIZON` override the defaults `5`, `1`, and `2`.

The compact transition workload enumerates engine-issued actions and applies
the checked-in parity path directly to `Game`, without receipts, state hashes,
or replay serialization. The replay workload runs the same fixture actions
through `Session` and verifies a second authoritative replay. Search clones one
in-memory root per legal action and follows the first canonically ordered action
for the configured horizon.

Output conforms to [rust-engine-benchmark.schema.json](rust-engine-benchmark.schema.json).
`peakRssBytes` is `null` when safe stdlib-only peak RSS measurement is not
available; Linux reports `/proc/self/status` `VmHWM`.

## Cutover baselines (2026-08-31)

| Release workload | TypeScript | Rust | Ratio |
| --- | ---: | ---: | ---: |
| Transitions/second | 362.222 | 603,642.483 | 1,666.5x |
| Search nodes/second (horizon 2) | 207.255 | 53,085.662 | 256.1x |
| Fully replayed and re-verified games/second | 0.706 | 1.930 | 2.73x |

The JSON baselines retain aggregate, median, and p95 measurements. Rust's
20-sample comparable run exceeds the mature transition target; authoritative
replay remains below the initial 3x target and is still an optimization gate.
Windows peak RSS is unavailable in the stdlib-only harness, so a final Linux
release run must supply that measurement before cutover.
