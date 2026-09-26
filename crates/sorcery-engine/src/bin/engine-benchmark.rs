use std::error::Error;
use std::hint::black_box;
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{Value, json};
use sorcery_engine::batch::{BatchJob, MAX_GAME_ACTIONS, run_game_batch};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::session::Session;
use sorcery_engine::simulator::{replay_selected, run_game};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const FIXTURE_JSON: &str =
    include_str!("../../../../tests/engine/fixtures/typescript-parity-v1.json");
const MAX_SAMPLES: u32 = 10_000;
const MAX_GAMES_PER_SAMPLE: u32 = 10_000;
const MAX_SEARCH_HORIZON: u32 = 32;
const MAX_SETUP_SAMPLES: u32 = 1_000;
const SETUP_SEEDS: [u32; 3] = [31, 37, 43];

type BenchmarkResult<T> = Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    action_ids: Vec<IdentityHash>,
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

struct PairedWorkload {
    candidate_as_north: Game,
    candidate_as_north_manifest_json: String,
    candidate_as_south: Game,
    candidate_as_south_manifest_json: String,
    policy: PolicySnapshot,
}

struct PairedSample {
    duration: Duration,
    latencies: Vec<Duration>,
    pairs: u32,
    transitions: u32,
}

struct BenchmarkConfiguration {
    games_per_sample: u32,
    sample_count: u32,
    search_horizon: u32,
    paired_rollouts_per_sample: u32,
}

struct SetupWorkload {
    label: &'static str,
    cards_per_zone: usize,
    card_definition_count: usize,
    manifests: Vec<String>,
}

struct SetupSample {
    game_duration: Duration,
    session_duration: Duration,
    manifests: u32,
}

fn main() -> BenchmarkResult<()> {
    if cfg!(debug_assertions) {
        return Err(io::Error::other(
            "engine benchmark requires `cargo run --release --locked -p sorcery-engine --bin engine-benchmark`",
        )
        .into());
    }
    if std::env::args().any(|arg| arg == "--worker-scaling") {
        let report = worker_scaling_report()?;
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, &report)?;
        writeln!(output)?;
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--setup-scaling") {
        let samples = positive_integer("BENCHMARK_SETUP_SAMPLES", 5, MAX_SETUP_SAMPLES)?;
        let report = setup_scaling_report(samples)?;
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, &report)?;
        writeln!(output)?;
        return Ok(());
    }

    let configuration = BenchmarkConfiguration {
        sample_count: positive_integer("BENCHMARK_SAMPLES", 5, MAX_SAMPLES)?,
        games_per_sample: positive_integer("BENCHMARK_GAMES_PER_SAMPLE", 1, MAX_GAMES_PER_SAMPLE)?,
        paired_rollouts_per_sample: positive_integer(
            "BENCHMARK_PAIRED_ROLLOUTS_PER_SAMPLE",
            1,
            MAX_GAMES_PER_SAMPLE,
        )?,
        search_horizon: positive_integer("BENCHMARK_SEARCH_HORIZON", 2, MAX_SEARCH_HORIZON)?,
    };
    let workloads = workloads()?;
    let paired_workload = paired_workload()?;

    black_box(transition_sample(&workloads[0], 1)?);
    black_box(search_sample(&workloads[0], 1)?);
    preflight_paired_replay(&paired_workload)?;

    let mut transitions = Vec::with_capacity(usize::try_from(configuration.sample_count)?);
    let mut game_replays = Vec::with_capacity(usize::try_from(configuration.sample_count)?);
    let mut search_nodes = Vec::with_capacity(usize::try_from(configuration.sample_count)?);
    let mut paired_rollouts = Vec::with_capacity(usize::try_from(configuration.sample_count)?);
    for sample_index in 0..configuration.sample_count {
        let workload = &workloads[usize::try_from(sample_index)? % workloads.len()];
        transitions.push(transition_sample(workload, configuration.games_per_sample)?);
        game_replays.push(game_replay_sample(
            workload,
            configuration.games_per_sample,
        )?);
        search_nodes.push(search_sample(workload, configuration.search_horizon)?);
        paired_rollouts.push(paired_rollout_sample(
            &paired_workload,
            configuration.paired_rollouts_per_sample,
        )?);
    }

    let report = report(
        &transitions,
        &game_replays,
        &search_nodes,
        &paired_rollouts,
        &configuration,
        &workloads,
    )?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &report)?;
    writeln!(output)?;
    Ok(())
}

fn setup_scaling_report(sample_count: u32) -> BenchmarkResult<Value> {
    let workloads = [
        setup_workload("small", 3, 3)?,
        setup_workload("large", 200, 200)?,
    ];
    for workload in &workloads {
        black_box(setup_sample(workload)?);
    }
    let samples = workloads
        .iter()
        .map(|workload| {
            let measurements = (0..sample_count)
                .map(|_| setup_sample(workload))
                .collect::<BenchmarkResult<Vec<_>>>()?;
            Ok((workload, measurements))
        })
        .collect::<BenchmarkResult<Vec<_>>>()?;
    let variants = samples
        .into_iter()
        .map(|(workload, measurements)| setup_summary(workload, &measurements))
        .collect::<Vec<_>>();
    let batch = batch_setup_summary(workloads.last().expect("large setup workload"))?;
    Ok(json!({
        "benchmarkVersion": 1,
        "mode": "setup-scaling",
        "sampleCount": sample_count,
        "seedCount": SETUP_SEEDS.len(),
        "largeBatch": batch,
        "variants": variants,
    }))
}

fn setup_workload(
    label: &'static str,
    atlas_count: usize,
    spellbook_count: usize,
) -> BenchmarkResult<SetupWorkload> {
    let manifests = SETUP_SEEDS
        .into_iter()
        .map(|seed| setup_manifest_json(atlas_count, spellbook_count, seed))
        .collect::<BenchmarkResult<Vec<_>>>()?;
    Ok(SetupWorkload {
        label,
        cards_per_zone: atlas_count.max(spellbook_count),
        card_definition_count: 2 + 2 * (atlas_count + spellbook_count),
        manifests,
    })
}

fn setup_manifest_json(
    atlas_count: usize,
    spellbook_count: usize,
    seed: u32,
) -> BenchmarkResult<String> {
    let mut cards = serde_json::Map::new();
    let north_atlas = setup_cards(&mut cards, "north", "site", atlas_count, false);
    let south_atlas = setup_cards(&mut cards, "south", "site", atlas_count, false);
    let north_spellbook = setup_cards(&mut cards, "north", "spell", spellbook_count, true);
    let south_spellbook = setup_cards(&mut cards, "south", "spell", spellbook_count, true);
    let mut manifest = json!({
        "authority": {
            "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "mode": "synthetic",
            "revisionId": "synthetic-setup-scaling-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": south_atlas,
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    cards.insert(
        "north-avatar".to_owned(),
        json!({
            "attack": 1,
            "cardType": "avatar",
            "defense": 1,
            "drawSpell": false,
            "life": 2,
        }),
    );
    cards.insert(
        "south-avatar".to_owned(),
        json!({
            "attack": 1,
            "cardType": "avatar",
            "defense": 1,
            "drawSpell": false,
            "life": 2,
        }),
    );
    manifest["cards"] = Value::Object(cards);
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    Ok(canonical_json(&manifest)?)
}

fn setup_cards(
    cards: &mut serde_json::Map<String, Value>,
    seat: &str,
    kind: &str,
    count: usize,
    authored: bool,
) -> Vec<String> {
    (1..=count)
        .map(|index| {
            let card_id = format!("{seat}-{kind}-{index}");
            let definition = if kind == "site" {
                json!({"cardType": "site", "elements": ["earth"]})
            } else if authored && index == 1 {
                json!({
                    "cardType": "magic",
                    "effectProgram": {
                        "effects": [
                            {
                                "alliedOnly": true,
                                "kind": "minion",
                                "op": "choose-unit",
                                "optional": false,
                                "relation": "anywhere",
                            },
                            {
                                "amount": 2,
                                "duration": "this-turn",
                                "modifier": "power",
                                "op": "grant",
                                "recipients": "chosen",
                            },
                            {"op": "draw-card"},
                        ],
                        "optionalSelection": false,
                        "selection": {
                            "kind": "unit",
                            "relation": "anywhere",
                            "unitKind": "minion",
                        },
                    },
                    "manaCost": 1,
                    "thresholds": {"air": 0, "earth": 1, "fire": 0, "water": 0},
                })
            } else {
                json!({
                    "attack": 5,
                    "cardType": "minion",
                    "defense": 1,
                    "manaCost": 1,
                    "thresholds": {"air": 0, "earth": 1, "fire": 0, "water": 0},
                })
            };
            cards.insert(card_id.clone(), definition);
            card_id
        })
        .collect()
}

fn setup_sample(workload: &SetupWorkload) -> BenchmarkResult<SetupSample> {
    let started = Instant::now();
    for manifest in &workload.manifests {
        black_box(Game::from_manifest_json(manifest)?);
    }
    let game_duration = started.elapsed();
    let started = Instant::now();
    for manifest in &workload.manifests {
        black_box(Session::new(manifest)?);
    }
    Ok(SetupSample {
        game_duration,
        session_duration: started.elapsed(),
        manifests: u32::try_from(workload.manifests.len())?,
    })
}

fn setup_summary(workload: &SetupWorkload, samples: &[SetupSample]) -> Value {
    let mut game_ms = samples
        .iter()
        .map(|sample| sample.game_duration.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    let mut session_ms = samples
        .iter()
        .map(|sample| sample.session_duration.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    game_ms.sort_by(f64::total_cmp);
    session_ms.sort_by(f64::total_cmp);
    let manifests = samples.first().map_or(0, |sample| sample.manifests);
    let manifest_count = f64::from(manifests);
    json!({
        "label": workload.label,
        "cardsPerZone": workload.cards_per_zone,
        "cardDefinitionCount": workload.card_definition_count,
        "manifestCount": manifests,
        "gameFromManifestJsonMedianMs": round(*percentile(&game_ms, 50)),
        "gameFromManifestJsonMedianPerManifestMs": round(*percentile(&game_ms, 50) / manifest_count),
        "sessionNewMedianMs": round(*percentile(&session_ms, 50)),
        "sessionNewMedianPerManifestMs": round(*percentile(&session_ms, 50) / manifest_count),
        "gameFromManifestJsonSamplesMs": game_ms.into_iter().map(round).collect::<Vec<_>>(),
        "sessionNewSamplesMs": session_ms.into_iter().map(round).collect::<Vec<_>>(),
    })
}

fn batch_setup_summary(workload: &SetupWorkload) -> BenchmarkResult<Value> {
    const JOB_COUNT: usize = 4;
    const WORKERS: [usize; 2] = [1, 2];
    const REPEATS: usize = 3;
    let policy = baseline_policy(&workload.manifests[0])?;
    let deck_id = policy.deck_id();
    let jobs = (0..JOB_COUNT)
        .map(|index| BatchJob {
            manifest_json: &workload.manifests[index % workload.manifests.len()],
            north_deck_id: deck_id,
            north_policy: &policy,
            south_deck_id: deck_id,
            south_policy: &policy,
        })
        .collect::<Vec<_>>();
    let mut deterministic_hash = None;
    let mut measurements = Vec::with_capacity(WORKERS.len());
    for workers in WORKERS {
        let warmup_hash = identity_hash(&serde_json::to_value(black_box(run_game_batch(
            &jobs, workers,
        )?))?)?;
        if let Some(expected) = &deterministic_hash {
            if expected != &warmup_hash {
                return Err(io::Error::other("setup batch changed with worker count").into());
            }
        } else {
            deterministic_hash = Some(warmup_hash);
        }
        let mut durations = Vec::with_capacity(REPEATS);
        let mut repeat_hashes = Vec::with_capacity(REPEATS);
        for _ in 0..REPEATS {
            let started = Instant::now();
            let results = run_game_batch(&jobs, workers)?;
            durations.push(started.elapsed().as_secs_f64() * 1_000.0);
            let result_hash = identity_hash(&serde_json::to_value(black_box(results))?)?;
            if deterministic_hash.as_ref() != Some(&result_hash) {
                return Err(io::Error::other("setup batch result was not deterministic").into());
            }
            repeat_hashes.push(result_hash);
        }
        durations.sort_by(f64::total_cmp);
        measurements.push(json!({
            "workers": workers,
            "medianMs": round(durations[REPEATS / 2]),
            "samplesMs": durations.into_iter().map(round).collect::<Vec<_>>(),
            "resultHashes": repeat_hashes,
        }));
    }
    Ok(json!({
        "cardsPerZone": workload.cards_per_zone,
        "jobs": JOB_COUNT,
        "repeats": REPEATS,
        "deterministicResultHash": deterministic_hash,
        "workers": measurements,
    }))
}

fn worker_scaling_report() -> BenchmarkResult<Value> {
    const JOB_COUNT: u32 = 48;
    const REPEATS: usize = 3;
    const WORKERS: [usize; 6] = [1, 2, 4, 8, 16, 24];
    let manifests = (1..=JOB_COUNT)
        .map(synthetic_demo_manifest_json)
        .collect::<Result<Vec<_>, _>>()?;
    let policy = baseline_policy(&manifests[0])?;
    let second_policy = baseline_policy(&manifests[1])?;
    if policy.deck_id() != second_policy.deck_id() {
        return Err(io::Error::other("synthetic scaling manifests changed deck identity").into());
    }
    let jobs = manifests
        .iter()
        .map(|manifest_json| BatchJob {
            manifest_json,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        })
        .collect::<Vec<_>>();
    let mut baseline_hash = None;
    let mut samples = Vec::with_capacity(WORKERS.len());
    for workers in WORKERS {
        let warmup_hash = {
            let warmup = run_game_batch(&jobs, workers)?;
            identity_hash(&serde_json::to_value(&warmup)?)?
        };
        if let Some(expected) = &baseline_hash {
            if expected != &warmup_hash {
                return Err(
                    io::Error::other("worker scaling changed deterministic batch output").into(),
                );
            }
        } else {
            baseline_hash = Some(warmup_hash);
        }
        let mut durations = Vec::with_capacity(REPEATS);
        let mut hashes = Vec::with_capacity(REPEATS);
        for _ in 0..REPEATS {
            let started = Instant::now();
            let results = run_game_batch(&jobs, workers)?;
            let elapsed = started.elapsed();
            let hash = identity_hash(&serde_json::to_value(&results)?)?;
            if baseline_hash.as_ref() != Some(&hash) {
                return Err(
                    io::Error::other("worker scaling changed deterministic batch output").into(),
                );
            }
            durations.push(elapsed.as_secs_f64() * 1_000.0);
            hashes.push(hash);
        }
        durations.sort_by(f64::total_cmp);
        let median_ms = durations[REPEATS / 2];
        samples.push(json!({
            "workers": workers,
            "durationMs": round(median_ms),
            "games": JOB_COUNT,
            "gamesPerSecond": round(f64::from(JOB_COUNT) / (median_ms / 1_000.0)),
            "sampleDurationsMs": durations.into_iter().map(round).collect::<Vec<_>>(),
            "resultHashes": hashes,
        }));
    }
    Ok(json!({
        "benchmarkVersion": 1,
        "mode": "worker-scaling",
        "jobCount": JOB_COUNT,
        "repeats": REPEATS,
        "logicalCpuCount": std::thread::available_parallelism()?.get(),
        "workers": samples,
        "deterministicResultHash": baseline_hash,
    }))
}

fn paired_workload() -> BenchmarkResult<PairedWorkload> {
    let candidate_as_north_manifest_json = synthetic_demo_manifest_json(31)?;
    let candidate_as_south_manifest_json = swapped_manifest(&candidate_as_north_manifest_json)?;
    let policy = baseline_policy(&candidate_as_north_manifest_json)?;
    Ok(PairedWorkload {
        candidate_as_north: Game::from_manifest_json(&candidate_as_north_manifest_json)?,
        candidate_as_north_manifest_json,
        candidate_as_south: Game::from_manifest_json(&candidate_as_south_manifest_json)?,
        candidate_as_south_manifest_json,
        policy,
    })
}

fn swapped_manifest(manifest_json: &str) -> BenchmarkResult<String> {
    let mut manifest: Value = serde_json::from_str(manifest_json)?;
    manifest
        .as_object_mut()
        .ok_or_else(|| io::Error::other("benchmark manifest must be an object"))?
        .remove("manifestId")
        .ok_or_else(|| io::Error::other("benchmark manifest lacks its identity"))?;
    let decks = manifest["decks"]
        .as_object_mut()
        .ok_or_else(|| io::Error::other("benchmark manifest lacks decks"))?;
    let north = decks
        .remove("north")
        .ok_or_else(|| io::Error::other("benchmark manifest lacks North deck"))?;
    let south = decks
        .remove("south")
        .ok_or_else(|| io::Error::other("benchmark manifest lacks South deck"))?;
    decks.insert("north".to_owned(), south);
    decks.insert("south".to_owned(), north);
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    Ok(canonical_json(&manifest)?)
}

fn baseline_policy(manifest_json: &str) -> BenchmarkResult<PolicySnapshot> {
    let manifest: Value = serde_json::from_str(manifest_json)?;
    let mut body = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "engineVersion": manifest["engineVersion"],
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": [
                "keep-mulligan", "play-site", "summon-minion", "preferred-draw",
                "powered-movement", "beneficial-tactic", "move-toward-enemy",
                "end-turn", "canonical-fallback"
            ]
        },
        "tieBreak": "canonical-action-order-v1"
    });
    body["policyId"] = json!(identity_hash(&body)?);
    Ok(parse_policy_snapshot(&canonical_json(&body)?)?)
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
    fixture
        .games
        .into_iter()
        .map(|game| {
            let manifest_json = synthetic_demo_manifest_json(game.seed)?;
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

fn preflight_paired_replay(workload: &PairedWorkload) -> BenchmarkResult<()> {
    for (game, manifest_json) in [
        (
            &workload.candidate_as_north,
            workload.candidate_as_north_manifest_json.as_str(),
        ),
        (
            &workload.candidate_as_south,
            workload.candidate_as_south_manifest_json.as_str(),
        ),
    ] {
        let rollout = run_game(
            game.clone(),
            &workload.policy,
            &workload.policy,
            MAX_GAME_ACTIONS,
        )?;
        if !rollout.is_terminal() {
            return Err(io::Error::other("paired rollout preflight was nonterminal").into());
        }
        black_box(replay_selected(manifest_json, &rollout)?);
    }
    Ok(())
}

fn paired_rollout_sample(
    workload: &PairedWorkload,
    pair_count: u32,
) -> BenchmarkResult<PairedSample> {
    let mut latencies = Vec::with_capacity(usize::try_from(pair_count)?);
    let mut transitions = 0_u32;
    for _ in 0..pair_count {
        let started = Instant::now();
        let north = run_game(
            workload.candidate_as_north.clone(),
            &workload.policy,
            &workload.policy,
            MAX_GAME_ACTIONS,
        )?;
        let south = run_game(
            workload.candidate_as_south.clone(),
            &workload.policy,
            &workload.policy,
            MAX_GAME_ACTIONS,
        )?;
        if !north.is_terminal() || !south.is_terminal() {
            return Err(io::Error::other("paired rollout benchmark was nonterminal").into());
        }
        transitions = transitions
            .checked_add(u32::try_from(
                north.action_indices().len() + south.action_indices().len(),
            )?)
            .ok_or_else(|| io::Error::other("paired transition count overflow"))?;
        black_box((north.outcome(), south.outcome()));
        latencies.push(started.elapsed());
    }
    Ok(PairedSample {
        duration: latencies.iter().sum(),
        latencies,
        pairs: pair_count,
        transitions,
    })
}

fn report(
    transitions: &[Sample],
    game_replays: &[Sample],
    search_nodes: &[Sample],
    paired_rollouts: &[PairedSample],
    configuration: &BenchmarkConfiguration,
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
        "benchmarkVersion": 2,
        "configuration": {
            "gamesPerSample": configuration.games_per_sample,
            "sampleCount": configuration.sample_count,
            "searchHorizon": configuration.search_horizon,
            "pairedRolloutRepetitionsPerSample": configuration.paired_rollouts_per_sample,
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
        "pairedRollout": summarize_paired_rollouts(paired_rollouts)?,
        "searchNodes": summarize(search_nodes)?,
        "transitions": summarize(transitions)?,
    }))
}

struct Totals {
    operations: u64,
    per_second: f64,
}

fn summarize_paired_rollouts(samples: &[PairedSample]) -> BenchmarkResult<Value> {
    validate_samples(
        samples.iter().map(|sample| (sample.duration, sample.pairs)),
        "paired rollout",
    )?;
    let duration: Duration = samples.iter().map(|sample| sample.duration).sum();
    let pairs = samples.iter().try_fold(0_u64, |sum, sample| {
        sum.checked_add(u64::from(sample.pairs))
            .ok_or_else(|| io::Error::other("paired self-play count overflow"))
    })?;
    let transitions = samples.iter().try_fold(0_u64, |sum, sample| {
        sum.checked_add(u64::from(sample.transitions))
            .ok_or_else(|| io::Error::other("paired transition count overflow"))
    })?;
    let mut latencies = samples
        .iter()
        .flat_map(|sample| sample.latencies.iter().copied())
        .collect::<Vec<_>>();
    let mut pair_throughputs = samples
        .iter()
        .map(|sample| f64::from(sample.pairs) / sample.duration.as_secs_f64())
        .collect::<Vec<_>>();
    let mut transition_throughputs = samples
        .iter()
        .map(|sample| f64::from(sample.transitions) / sample.duration.as_secs_f64())
        .collect::<Vec<_>>();
    latencies.sort_unstable();
    pair_throughputs.sort_by(f64::total_cmp);
    transition_throughputs.sort_by(f64::total_cmp);
    let median_pairs_per_second = *percentile(&pair_throughputs, 50);
    let median_transitions_per_second = *percentile(&transition_throughputs, 50);
    let estimated_pairs = floor_nonnegative(median_pairs_per_second * 60.0)?;
    let estimated_games = estimated_pairs
        .checked_mul(2)
        .ok_or_else(|| io::Error::other("estimated game capacity overflow"))?;
    let estimated_transitions = floor_nonnegative(median_transitions_per_second * 60.0)?;
    Ok(json!({
        "aggregate": {
            "durationMs": round(duration.as_secs_f64() * 1_000.0),
            "games": pairs * 2,
            "pairs": pairs,
            "transitions": transitions,
        },
        "estimated60SecondCapacity": {
            "games": estimated_games,
            "method": "singleThreadShortSampleEstimate",
            "pairs": estimated_pairs,
            "transitions": estimated_transitions,
            "workload": "twoSeatSpeculativeRolloutOnly",
        },
        "replayPreflightOrientations": 2,
        "seed": 31,
        "pairLatencyMs": {
            "median": round(percentile(&latencies, 50).as_secs_f64() * 1_000.0),
            "p95": round(percentile(&latencies, 95).as_secs_f64() * 1_000.0),
        },
        "throughputPerSecond": {
            "gamesMedian": round(median_pairs_per_second * 2.0),
            "gamesP95": round(*percentile(&pair_throughputs, 95) * 2.0),
            "pairsMedian": round(median_pairs_per_second),
            "pairsP95": round(*percentile(&pair_throughputs, 95)),
            "transitionsMedian": round(median_transitions_per_second),
            "transitionsP95": round(*percentile(&transition_throughputs, 95)),
        },
        "samples": samples.iter().map(|sample| json!({
            "durationMs": round(sample.duration.as_secs_f64() * 1_000.0),
            "games": u64::from(sample.pairs) * 2,
            "pairs": sample.pairs,
            "transitions": sample.transitions,
        })).collect::<Vec<_>>(),
        "uniqueSeeds": 1,
    }))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn floor_nonnegative(value: f64) -> BenchmarkResult<u64> {
    const U64_MAX_PLUS_ONE: f64 = 18_446_744_073_709_551_616.0;
    if !(0.0..U64_MAX_PLUS_ONE).contains(&value) {
        return Err(io::Error::other("benchmark estimate is outside the u64 range").into());
    }
    Ok(value.floor() as u64)
}

fn validate_samples(
    samples: impl Iterator<Item = (Duration, u32)>,
    label: &str,
) -> BenchmarkResult<()> {
    let samples = samples.collect::<Vec<_>>();
    if samples.is_empty()
        || samples
            .iter()
            .any(|(duration, operations)| duration.is_zero() || *operations == 0)
    {
        return Err(io::Error::other(format!(
            "{label} samples must be nonempty with positive work and duration"
        ))
        .into());
    }
    Ok(())
}

fn totals(samples: &[Sample]) -> BenchmarkResult<Totals> {
    validate_samples(
        samples
            .iter()
            .map(|sample| (sample.duration, sample.operations)),
        "benchmark",
    )?;
    let duration: Duration = samples.iter().map(|sample| sample.duration).sum();
    let operations = samples.iter().try_fold(0_u64, |sum, sample| {
        sum.checked_add(u64::from(sample.operations))
            .ok_or_else(|| io::Error::other("benchmark operation count overflow"))
    })?;
    let operations_f64 = samples
        .iter()
        .map(|sample| f64::from(sample.operations))
        .sum::<f64>();
    Ok(Totals {
        operations,
        per_second: operations_f64 / duration.as_secs_f64(),
    })
}

fn summarize(samples: &[Sample]) -> BenchmarkResult<Value> {
    validate_samples(
        samples
            .iter()
            .map(|sample| (sample.duration, sample.operations)),
        "benchmark",
    )?;
    let duration: Duration = samples.iter().map(|sample| sample.duration).sum();
    let operations = samples.iter().try_fold(0_u64, |sum, sample| {
        sum.checked_add(u64::from(sample.operations))
            .ok_or_else(|| io::Error::other("benchmark operation count overflow"))
    })?;
    let operations_f64 = samples
        .iter()
        .map(|sample| f64::from(sample.operations))
        .sum::<f64>();
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
            "perSecond": round(operations_f64 / duration.as_secs_f64()),
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
    use super::{PairedSample, Sample, summarize, summarize_paired_rollouts};
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

    #[test]
    fn paired_summary_should_report_auditable_sixty_second_estimate() {
        let samples = [PairedSample {
            duration: Duration::from_millis(100),
            latencies: vec![Duration::from_millis(50); 2],
            pairs: 2,
            transitions: 20,
        }];

        let report = summarize_paired_rollouts(&samples).expect("paired summary");

        assert_eq!(
            report["estimated60SecondCapacity"],
            json!({
                "games": 2_400,
                "method": "singleThreadShortSampleEstimate",
                "pairs": 1_200,
                "transitions": 12_000,
                "workload": "twoSeatSpeculativeRolloutOnly",
            })
        );
        assert_eq!(report["samples"][0]["games"], 4);
    }

    #[test]
    fn summaries_should_reject_empty_or_zero_duration_samples() {
        assert!(summarize(&[]).is_err());
        assert!(
            summarize_paired_rollouts(&[PairedSample {
                duration: Duration::ZERO,
                latencies: vec![Duration::ZERO],
                pairs: 1,
                transitions: 1,
            }])
            .is_err()
        );
    }
}
