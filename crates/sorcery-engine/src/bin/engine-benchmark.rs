use std::error::Error;
use std::hint::black_box;
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::game::Game;
use sorcery_engine::session::Session;

const FIXTURE_JSON: &str =
    include_str!("../../../../tests/engine/fixtures/typescript-parity-v1.json");
const MAX_SAMPLES: u32 = 10_000;
const MAX_GAMES_PER_SAMPLE: u32 = 10_000;
const MAX_SEARCH_HORIZON: u32 = 32;

type BenchmarkResult<T> = Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    action_ids: Vec<IdentityHash>,
    manifest_json: Option<String>,
    seed: u32,
}

struct Workload {
    action_ids: Vec<IdentityHash>,
    action_indices: Vec<usize>,
    manifest_json: String,
    seed: u32,
}

struct Sample {
    duration: Duration,
    latencies: Vec<Duration>,
    operations: u32,
}

fn main() -> BenchmarkResult<()> {
    if cfg!(debug_assertions) {
        return Err(io::Error::other(
            "engine benchmark requires `cargo run --release --locked -p sorcery-engine --bin engine-benchmark`",
        )
        .into());
    }

    let sample_count = positive_integer("BENCHMARK_SAMPLES", 5, MAX_SAMPLES)?;
    let games_per_sample = positive_integer("BENCHMARK_GAMES_PER_SAMPLE", 1, MAX_GAMES_PER_SAMPLE)?;
    let search_horizon = positive_integer("BENCHMARK_SEARCH_HORIZON", 2, MAX_SEARCH_HORIZON)?;
    let workloads = workloads()?;

    black_box(transition_sample(&workloads[0], 1)?);
    black_box(search_sample(&workloads[0], 1)?);

    let mut transitions = Vec::with_capacity(usize::try_from(sample_count)?);
    let mut game_replays = Vec::with_capacity(usize::try_from(sample_count)?);
    let mut search_nodes = Vec::with_capacity(usize::try_from(sample_count)?);
    for sample_index in 0..sample_count {
        let workload = &workloads[usize::try_from(sample_index)? % workloads.len()];
        transitions.push(transition_sample(workload, games_per_sample)?);
        game_replays.push(game_replay_sample(workload, games_per_sample)?);
        search_nodes.push(search_sample(workload, search_horizon)?);
    }

    let report = report(
        &transitions,
        &game_replays,
        &search_nodes,
        sample_count,
        games_per_sample,
        search_horizon,
        &workloads,
    )?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &report)?;
    writeln!(output)?;
    Ok(())
}

fn positive_integer(name: &str, fallback: u32, maximum: u32) -> BenchmarkResult<u32> {
    let value = match std::env::var(name) {
        Ok(value) => value.parse::<u32>()?,
        Err(std::env::VarError::NotPresent) => fallback,
        Err(error) => return Err(error.into()),
    };
    if value == 0 || value > maximum {
        return Err(io::Error::other(format!(
            "{name} must be an integer from 1 through {maximum}"
        ))
        .into());
    }
    Ok(value)
}

fn workloads() -> BenchmarkResult<Vec<Workload>> {
    let fixture: Fixture = serde_json::from_str(FIXTURE_JSON)?;
    let template = fixture
        .games
        .iter()
        .find_map(|game| game.manifest_json.as_deref())
        .ok_or_else(|| io::Error::other("parity fixture has no canonical manifest"))?
        .to_owned();
    fixture
        .games
        .into_iter()
        .map(|game| {
            let manifest_json = manifest_for_seed(&template, game.seed)?;
            let action_indices = resolve_action_indices(&manifest_json, &game.action_ids)?;
            Ok(Workload {
                action_ids: game.action_ids,
                action_indices,
                manifest_json,
                seed: game.seed,
            })
        })
        .collect()
}

fn resolve_action_indices(
    manifest_json: &str,
    action_ids: &[IdentityHash],
) -> BenchmarkResult<Vec<usize>> {
    let mut game = Game::from_manifest_json(manifest_json)?;
    let mut indices = Vec::with_capacity(action_ids.len());
    for expected_action_id in action_ids {
        let actions = game.legal_actions()?;
        let mut selected = None;
        for (index, action) in actions.iter().enumerate() {
            if action.to_legal_action()?.action_id == *expected_action_id {
                selected = Some(index);
                break;
            }
        }
        let index = selected.ok_or_else(|| io::Error::other("fixture action is not legal"))?;
        game.apply_action(&actions[index])?;
        indices.push(index);
    }
    Ok(indices)
}

fn manifest_for_seed(template: &str, seed: u32) -> BenchmarkResult<String> {
    let mut manifest: Value = serde_json::from_str(template)?;
    let body = manifest
        .as_object_mut()
        .ok_or_else(|| io::Error::other("fixture manifest must be an object"))?;
    body.remove("manifestId")
        .ok_or_else(|| io::Error::other("fixture manifest has no identity"))?;
    body.insert("seed".to_owned(), json!(seed));
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    Ok(canonical_json(&manifest)?)
}

fn transition_sample(workload: &Workload, game_count: u32) -> BenchmarkResult<Sample> {
    let mut latencies = Vec::new();
    for _ in 0..game_count {
        let mut game = Game::from_manifest_json(&workload.manifest_json)?;
        for &action_index in &workload.action_indices {
            let started = Instant::now();
            let actions = game.legal_actions()?;
            let action = actions
                .get(action_index)
                .ok_or_else(|| io::Error::other("fixture action index is not legal"))?;
            black_box(game.apply_action(action)?);
            latencies.push(started.elapsed());
        }
    }
    let operations = u32::try_from(latencies.len())?;
    Ok(Sample {
        duration: latencies.iter().sum(),
        latencies,
        operations,
    })
}

fn game_replay_sample(workload: &Workload, game_count: u32) -> BenchmarkResult<Sample> {
    let mut latencies = Vec::with_capacity(usize::try_from(game_count)?);
    for _ in 0..game_count {
        let started = Instant::now();
        let session = Session::replay(&workload.manifest_json, &workload.action_ids)?;
        if !session.verify_replay()? {
            return Err(io::Error::other("authoritative replay verification failed").into());
        }
        black_box(session.transcript());
        latencies.push(started.elapsed());
    }
    Ok(Sample {
        duration: latencies.iter().sum(),
        latencies,
        operations: game_count,
    })
}

fn search_sample(workload: &Workload, horizon: u32) -> BenchmarkResult<Sample> {
    let root = Game::from_manifest_json(&workload.manifest_json)?;
    let started = Instant::now();
    let root_actions = root.legal_actions()?;
    let mut operations = 0_u32;
    for root_action in root_actions {
        let mut branch = root.clone();
        black_box(branch.apply_action(&root_action)?);
        operations = operations
            .checked_add(1)
            .ok_or_else(|| io::Error::other("search node count overflow"))?;
        for _ in 0..horizon {
            let actions = branch.legal_actions()?;
            let Some(action) = actions.first() else {
                break;
            };
            black_box(branch.apply_action(action)?);
            operations = operations
                .checked_add(1)
                .ok_or_else(|| io::Error::other("search node count overflow"))?;
        }
    }
    Ok(Sample {
        duration: started.elapsed(),
        latencies: Vec::new(),
        operations,
    })
}

fn report(
    transitions: &[Sample],
    game_replays: &[Sample],
    search_nodes: &[Sample],
    sample_count: u32,
    games_per_sample: u32,
    search_horizon: u32,
    workloads: &[Workload],
) -> BenchmarkResult<Value> {
    let transition_totals = totals(transitions)?;
    let replay_totals = totals(game_replays)?;
    let search_totals = totals(search_nodes)?;
    let peak_rss_bytes = peak_rss_bytes();
    let measured_at_unix_ms =
        u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    Ok(json!({
        "aggregate": {
            "games": replay_totals.operations,
            "gamesPerSecond": round(replay_totals.per_second),
            "peakRssBytes": peak_rss_bytes,
            "searchNodes": search_totals.operations,
            "searchNodesPerSecond": round(search_totals.per_second),
            "transitions": transition_totals.operations,
            "transitionsPerSecond": round(transition_totals.per_second),
        },
        "benchmarkVersion": 1,
        "configuration": {
            "gamesPerSample": games_per_sample,
            "sampleCount": sample_count,
            "searchHorizon": search_horizon,
            "seeds": workloads.iter().map(|workload| workload.seed).collect::<Vec<_>>(),
        },
        "environment": {
            "arch": std::env::consts::ARCH,
            "cpu": std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unavailable".to_owned()),
            "logicalCpuCount": std::thread::available_parallelism()?.get(),
            "mode": "rust-release",
            "os": std::env::consts::OS,
            "peakRssMeasurement": if peak_rss_bytes.is_some() {
                "linux-proc-vmhwm"
            } else {
                "unavailable-without-unsafe-or-dependency"
            },
            "totalMemoryBytes": Value::Null,
        },
        "gameReplay": summarize(game_replays)?,
        "measuredAtUnixMs": measured_at_unix_ms,
        "searchNodes": summarize(search_nodes)?,
        "transitions": summarize(transitions)?,
    }))
}

struct Totals {
    operations: u32,
    per_second: f64,
}

fn totals(samples: &[Sample]) -> BenchmarkResult<Totals> {
    let duration: Duration = samples.iter().map(|sample| sample.duration).sum();
    let operations = samples.iter().try_fold(0_u32, |sum, sample| {
        sum.checked_add(sample.operations)
            .ok_or_else(|| io::Error::other("benchmark operation count overflow"))
    })?;
    Ok(Totals {
        operations,
        per_second: f64::from(operations) / duration.as_secs_f64(),
    })
}

fn summarize(samples: &[Sample]) -> BenchmarkResult<Value> {
    let duration: Duration = samples.iter().map(|sample| sample.duration).sum();
    let operations = samples.iter().try_fold(0_u32, |sum, sample| {
        sum.checked_add(sample.operations)
            .ok_or_else(|| io::Error::other("benchmark operation count overflow"))
    })?;
    let mut latencies: Vec<_> = samples
        .iter()
        .flat_map(|sample| sample.latencies.iter().copied())
        .collect();
    if latencies.is_empty() {
        latencies.extend(
            samples
                .iter()
                .map(|sample| sample.duration.div_f64(f64::from(sample.operations))),
        );
    }
    let mut throughputs: Vec<_> = samples
        .iter()
        .map(|sample| f64::from(sample.operations) / sample.duration.as_secs_f64())
        .collect();
    latencies.sort_unstable();
    throughputs.sort_by(f64::total_cmp);
    Ok(json!({
        "aggregate": {
            "durationMs": round(duration.as_secs_f64() * 1_000.0),
            "operations": operations,
            "perSecond": round(f64::from(operations) / duration.as_secs_f64()),
        },
        "latencyMs": {
            "median": round(percentile(&latencies, 50).as_secs_f64() * 1_000.0),
            "p95": round(percentile(&latencies, 95).as_secs_f64() * 1_000.0),
        },
        "throughputPerSecond": {
            "median": round(*percentile(&throughputs, 50)),
            "p95": round(*percentile(&throughputs, 95)),
        },
    }))
}

fn percentile<T>(sorted: &[T], percentage: usize) -> &T {
    let index = sorted.len().saturating_mul(percentage).div_ceil(100) - 1;
    &sorted[index]
}

fn round(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

#[cfg(target_os = "linux")]
fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kib = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?;
    kib.checked_mul(1_024)
}

#[cfg(not(target_os = "linux"))]
const fn peak_rss_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::{Sample, summarize};
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn summary_should_report_nearest_rank_percentiles() {
        let samples = [Sample {
            duration: Duration::from_millis(100),
            latencies: vec![
                Duration::from_millis(1),
                Duration::from_millis(2),
                Duration::from_millis(3),
                Duration::from_millis(4),
            ],
            operations: 4,
        }];

        let report = summarize(&samples).expect("valid summary");

        assert_eq!(report["latencyMs"], json!({ "median": 2.0, "p95": 4.0 }));
    }
}
