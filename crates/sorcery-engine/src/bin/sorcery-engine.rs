use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sorcery_engine::batch::{
    BatchJob, GameBatchResult, MAX_BATCH_BYTES, MAX_BATCH_JOBS, MAX_BATCH_WORKERS,
    default_batch_workers, run_game_batch,
};
use sorcery_engine::canonical::{
    IdentityHash, canonical_json, canonical_json_allowing_finite_floats, identity_hash,
    parse_json_without_duplicate_keys,
};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, FormatContext, OfficialCardMapping,
    validate_deck,
};
use sorcery_engine::gauntlet::{GauntletOrientation, GauntletPair, GauntletReport, run_gauntlet};
use sorcery_engine::novelty::NOVELTY_ACTION_LIMIT;
use sorcery_engine::novelty_frontier::{
    FrontierJob, frontier_response_value, run_private_novelty_frontier,
};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const SYNTHETIC_DECK_ID: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MAX_BATCH_JSON_BYTES: usize = MAX_BATCH_BYTES * 2 + 1024 * 1024;

type CliResult<T> = Result<T, Box<dyn Error>>;

enum Command {
    Demo { seed: u32 },
    Batch { workers: usize, seeds: Vec<u32> },
    BatchJson,
    GauntletJson,
    NoveltyGauntletJson,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BatchJsonRequest {
    schema_version: u8,
    workers: usize,
    jobs: Vec<BatchJsonJob>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BatchJsonJob {
    manifest_json: String,
    north_deck_id: IdentityHash,
    north_policy: Value,
    south_deck_id: IdentityHash,
    south_policy: Value,
}

struct ValidatedBatchJsonJob {
    manifest_json: String,
    north_deck_id: IdentityHash,
    north_policy: PolicySnapshot,
    south_deck_id: IdentityHash,
    south_policy: PolicySnapshot,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GauntletJsonRequest {
    deck_a_id: String,
    deck_b_id: String,
    pairs: Vec<GauntletJsonPair>,
    schema_version: u8,
    workers: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GauntletJsonPair {
    a_north: BatchJsonJob,
    b_north: BatchJsonJob,
    seed: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NoveltyGauntletJsonRequest {
    jobs: Vec<NoveltyGauntletJsonJob>,
    max_actions: u64,
    schema_version: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NoveltyGauntletJsonJob {
    job_id: String,
    lesson_id: String,
    manifest_json: String,
    orientation: String,
}

struct ValidatedGauntletPair {
    a_north: ValidatedBatchJsonJob,
    b_north: ValidatedBatchJsonJob,
    seed: u32,
}

#[derive(Deserialize)]
struct ManifestDeckEnvelope {
    cards: BTreeMap<String, ManifestCard>,
    decks: ManifestDecks,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestCard {
    card_type: CardType,
    token: Option<bool>,
}

#[derive(Deserialize)]
struct ManifestDecks {
    north: ManifestDeck,
    south: ManifestDeck,
}

#[derive(Deserialize)]
struct ManifestDeck {
    atlas: Vec<String>,
    avatar: String,
    spellbook: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> CliResult<()> {
    match parse_args(std::env::args().skip(1))? {
        Command::Demo { seed } => {
            let mut results = run_synthetic_batch(&[seed], 1)?;
            let report = results
                .pop()
                .ok_or_else(|| io::Error::other("demo produced no result"))?
                .report;
            write_canonical_json(&report)
        }
        Command::Batch { workers, seeds } => {
            write_canonical_json(&run_synthetic_batch(&seeds, workers)?)
        }
        Command::BatchJson => {
            let input = read_batch_json_stdin()?;
            write_canonical_json(&run_batch_json(&input)?)
        }
        Command::GauntletJson => {
            let input = read_batch_json_stdin()?;
            write_report_json(&run_gauntlet_json(&input)?)
        }
        Command::NoveltyGauntletJson => {
            let input = read_batch_json_stdin()?;
            write_canonical_json(&run_novelty_gauntlet_json(&input)?)
        }
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> CliResult<Command> {
    let mut args = args;
    match args.next().as_deref() {
        Some("demo") => {
            let seed = args.next().map_or(Ok(1), |value| parse_seed(&value))?;
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine demo [seed]").into());
            }
            Ok(Command::Demo { seed })
        }
        Some("batch") => {
            let workers = args.next().map_or_else(
                || Ok(default_batch_workers()),
                |value| parse_workers(&value),
            )?;
            let mut seeds = args
                .map(|value| parse_seed(&value))
                .collect::<Result<Vec<_>, _>>()?;
            if seeds.is_empty() {
                seeds.push(1);
            }
            if seeds.len() > MAX_BATCH_JOBS {
                return Err(io::Error::other("batch must contain 1-256 seeds").into());
            }
            Ok(Command::Batch { workers, seeds })
        }
        Some("batch-json") => {
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine batch-json").into());
            }
            Ok(Command::BatchJson)
        }
        Some("gauntlet-json") => {
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine gauntlet-json").into());
            }
            Ok(Command::GauntletJson)
        }
        Some("novelty-gauntlet-json") => {
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine novelty-gauntlet-json").into());
            }
            Ok(Command::NoveltyGauntletJson)
        }
        _ => Err(io::Error::other(
            "usage: sorcery-engine demo [seed] | batch [workers] [seeds...] | batch-json | gauntlet-json | novelty-gauntlet-json",
        )
        .into()),
    }
}

fn read_batch_json_stdin() -> CliResult<Vec<u8>> {
    let mut input = Vec::new();
    io::stdin()
        .lock()
        .take(u64::try_from(MAX_BATCH_JSON_BYTES)? + 1)
        .read_to_end(&mut input)?;
    validate_batch_json_size(input.len())?;
    Ok(input)
}

fn validate_batch_json_size(bytes: usize) -> CliResult<()> {
    if bytes > MAX_BATCH_JSON_BYTES {
        return Err(io::Error::other("batch-json input exceeds 129 MiB").into());
    }
    Ok(())
}

fn validate_batch_json_job(job: BatchJsonJob, label: &str) -> CliResult<ValidatedBatchJsonJob> {
    let (north_manifest_deck_id, south_manifest_deck_id) = manifest_deck_ids(&job.manifest_json)?;
    if job.north_deck_id != north_manifest_deck_id || job.south_deck_id != south_manifest_deck_id {
        return Err(io::Error::other(format!(
            "{label} deck IDs do not match manifest deck composition"
        ))
        .into());
    }
    Ok(ValidatedBatchJsonJob {
        manifest_json: job.manifest_json,
        north_deck_id: job.north_deck_id,
        north_policy: parse_policy_snapshot(&canonical_json(&job.north_policy)?)?,
        south_deck_id: job.south_deck_id,
        south_policy: parse_policy_snapshot(&canonical_json(&job.south_policy)?)?,
    })
}

fn batch_job_from_validated(job: &ValidatedBatchJsonJob) -> BatchJob<'_> {
    BatchJob {
        manifest_json: &job.manifest_json,
        north_deck_id: &job.north_deck_id,
        north_policy: &job.north_policy,
        south_deck_id: &job.south_deck_id,
        south_policy: &job.south_policy,
    }
}

fn run_batch_json(input: &[u8]) -> CliResult<Vec<GameBatchResult>> {
    validate_batch_json_size(input.len())?;
    let text = std::str::from_utf8(input)?;
    let value = parse_json_without_duplicate_keys(text)?;
    let request: BatchJsonRequest = serde_json::from_value(value)?;
    if request.schema_version != 1 {
        return Err(io::Error::other("batch-json schemaVersion must be 1").into());
    }
    if !(1..=MAX_BATCH_WORKERS).contains(&request.workers) {
        return Err(io::Error::other("batch-json workers must be 1-8").into());
    }
    if request.jobs.is_empty() || request.jobs.len() > MAX_BATCH_JOBS {
        return Err(io::Error::other("batch-json must contain 1-256 jobs").into());
    }
    let validated = request
        .jobs
        .into_iter()
        .map(|job| validate_batch_json_job(job, "batch-json"))
        .collect::<CliResult<Vec<_>>>()?;
    let jobs = validated
        .iter()
        .map(batch_job_from_validated)
        .collect::<Vec<_>>();
    Ok(run_game_batch(&jobs, request.workers)?)
}

fn run_gauntlet_json(input: &[u8]) -> CliResult<GauntletReport> {
    validate_batch_json_size(input.len())?;
    let text = std::str::from_utf8(input)?;
    let value = parse_json_without_duplicate_keys(text)?;
    let request: GauntletJsonRequest = serde_json::from_value(value)?;
    if request.schema_version != 1 {
        return Err(io::Error::other("gauntlet-json schemaVersion must be 1").into());
    }
    if !(1..=MAX_BATCH_WORKERS).contains(&request.workers) {
        return Err(io::Error::other("gauntlet-json workers must be 1-8").into());
    }
    if request.pairs.is_empty() || request.pairs.len() > MAX_BATCH_JOBS / 2 {
        return Err(io::Error::other("gauntlet-json must contain 1-128 seed pairs").into());
    }
    if request.deck_a_id.trim().is_empty()
        || request.deck_b_id.trim().is_empty()
        || request.deck_a_id == request.deck_b_id
    {
        return Err(
            io::Error::other("gauntlet-json deck IDs must be distinct nonempty strings").into(),
        );
    }
    let validated = request
        .pairs
        .into_iter()
        .map(|pair| {
            Ok(ValidatedGauntletPair {
                a_north: validate_batch_json_job(pair.a_north, "gauntlet-json")?,
                b_north: validate_batch_json_job(pair.b_north, "gauntlet-json")?,
                seed: pair.seed,
            })
        })
        .collect::<CliResult<Vec<_>>>()?;
    let pairs = validated
        .iter()
        .map(|pair| GauntletPair {
            seed: pair.seed,
            orientations: [
                GauntletOrientation {
                    job: batch_job_from_validated(&pair.a_north),
                    north_deck_id: request.deck_a_id.as_str(),
                    south_deck_id: request.deck_b_id.as_str(),
                },
                GauntletOrientation {
                    job: batch_job_from_validated(&pair.b_north),
                    north_deck_id: request.deck_b_id.as_str(),
                    south_deck_id: request.deck_a_id.as_str(),
                },
            ],
        })
        .collect::<Vec<_>>();
    Ok(run_gauntlet(&pairs, request.workers)?)
}

fn run_novelty_gauntlet_json(input: &[u8]) -> CliResult<Value> {
    validate_batch_json_size(input.len())?;
    let text = std::str::from_utf8(input)?;
    let value = parse_json_without_duplicate_keys(text)?;
    let request: NoveltyGauntletJsonRequest = serde_json::from_value(value)?;
    if request.schema_version != 1 {
        return Err(io::Error::other("novelty-gauntlet-json schemaVersion must be 1").into());
    }
    if request.max_actions > NOVELTY_ACTION_LIMIT {
        return Err(io::Error::other("novelty-gauntlet-json maxActions must be 0-500").into());
    }
    if request.jobs.len() != 4 {
        return Err(io::Error::other("novelty-gauntlet-json must contain exactly 4 jobs").into());
    }
    for job in &request.jobs {
        if job.job_id.trim().is_empty()
            || job.lesson_id.trim().is_empty()
            || job.orientation.trim().is_empty()
            || job.manifest_json.trim().is_empty()
        {
            return Err(io::Error::other(
                "novelty-gauntlet-json jobs require nonempty jobId, lessonId, orientation, and manifestJson",
            )
            .into());
        }
    }
    let jobs = request
        .jobs
        .into_iter()
        .map(|job| FrontierJob {
            job_id: job.job_id,
            lesson_id: job.lesson_id,
            orientation: job.orientation,
            manifest_json: job.manifest_json,
        })
        .collect::<Vec<_>>();
    let run = run_private_novelty_frontier(&jobs, request.max_actions)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(frontier_response_value(&run))
}

fn manifest_deck_ids(manifest_json: &str) -> CliResult<(IdentityHash, IdentityHash)> {
    let manifest: ManifestDeckEnvelope = serde_json::from_str(manifest_json)?;
    let catalog = manifest
        .cards
        .into_iter()
        .map(|(stable_id, card)| CardCatalogEntry {
            stable_id,
            card_type: card.card_type,
            rarity: None,
            engine_supported: true,
            official_mapping: OfficialCardMapping::Unavailable,
            token: card.token.unwrap_or(false),
        })
        .collect::<Vec<_>>();
    let north = manifest_deck_id(manifest.decks.north, &catalog)?;
    let south = manifest_deck_id(manifest.decks.south, &catalog)?;
    Ok((north, south))
}

fn manifest_deck_id(deck: ManifestDeck, catalog: &[CardCatalogEntry]) -> CliResult<IdentityHash> {
    let validation = validate_deck(
        CandidateDeck {
            avatar: deck.avatar,
            atlas: counted_cards(deck.atlas)?,
            spellbook: counted_cards(deck.spellbook)?,
        },
        catalog,
        FormatContext::constructed(),
    )?;
    Ok(validation.deck_id().clone())
}

fn counted_cards(card_ids: Vec<String>) -> CliResult<Vec<CardCount>> {
    let mut counts = BTreeMap::<String, u32>::new();
    for card_id in card_ids {
        let copies = counts.entry(card_id).or_default();
        *copies = copies
            .checked_add(1)
            .ok_or_else(|| io::Error::other("manifest deck copy count overflowed"))?;
    }
    Ok(counts
        .into_iter()
        .map(|(card_id, copies)| CardCount { card_id, copies })
        .collect())
}

fn parse_seed(value: &str) -> CliResult<u32> {
    value
        .parse()
        .map_err(|_| io::Error::other("seed must be an unsigned 32-bit integer").into())
}

fn parse_workers(value: &str) -> CliResult<usize> {
    let workers = value
        .parse::<usize>()
        .map_err(|_| io::Error::other("workers must be an integer from 1 through 8"))?;
    if !(1..=MAX_BATCH_WORKERS).contains(&workers) {
        return Err(io::Error::other("workers must be an integer from 1 through 8").into());
    }
    Ok(workers)
}

fn run_synthetic_batch(seeds: &[u32], workers: usize) -> CliResult<Vec<GameBatchResult>> {
    let manifests = seeds
        .iter()
        .map(|&seed| synthetic_demo_manifest_json(seed))
        .collect::<Result<Vec<_>, _>>()?;
    let policy = baseline_policy(&manifests[0])?;
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
    Ok(run_game_batch(&jobs, workers)?)
}

fn baseline_policy(manifest_json: &str) -> CliResult<PolicySnapshot> {
    policy_for_deck(manifest_json, SYNTHETIC_DECK_ID)
}

fn policy_for_deck(manifest_json: &str, deck_id: &str) -> CliResult<PolicySnapshot> {
    let manifest: Value = serde_json::from_str(manifest_json)?;
    let mut body = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": deck_id,
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

fn write_canonical_json(value: &impl Serialize) -> CliResult<()> {
    let output = canonical_json(&serde_json::to_value(value)?)?;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{output}")?;
    Ok(())
}

/// Reports may include finite float averages; identity JSON still stays integer-only.
fn write_report_json(value: &impl Serialize) -> CliResult<()> {
    let mut value = serde_json::to_value(value)?;
    normalize_whole_number_field(&mut value, "averageTurns");
    let output = canonical_json_allowing_finite_floats(&value)?;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{output}")?;
    Ok(())
}

fn normalize_whole_number_field(value: &mut Value, field: &str) {
    let Some(Value::Number(number)) = value.get_mut(field) else {
        return;
    };
    let Some(float) = number.as_f64().filter(|value| value.is_finite()) else {
        return;
    };
    if float.fract() != 0.0 {
        return;
    }
    let rendered = format!("{float:.0}");
    if let Ok(as_u64) = rendered.parse::<u64>() {
        *number = serde_json::Number::from(as_u64);
    } else if let Ok(as_i64) = rendered.parse::<i64>() {
        *number = serde_json::Number::from(as_i64);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sorcery_engine::canonical::canonical_json;
    use sorcery_engine::synthetic::synthetic_demo_manifest_json;

    use super::{
        Command, MAX_BATCH_JSON_BYTES, manifest_deck_ids, parse_args, policy_for_deck,
        run_batch_json, validate_batch_json_size,
    };

    #[test]
    fn parse_args_should_apply_batch_defaults() {
        let command = parse_args(["batch".to_owned()].into_iter()).expect("valid defaults");
        let Command::Batch { seeds, workers } = command else {
            panic!("expected batch command");
        };

        assert_eq!((seeds, (1..=8).contains(&workers)), (vec![1], true));
    }

    #[test]
    fn parse_args_should_reject_out_of_range_workers() {
        let error = parse_args(["batch".to_owned(), "9".to_owned()].into_iter())
            .err()
            .expect("invalid workers");

        assert_eq!(
            error.to_string(),
            "workers must be an integer from 1 through 8"
        );
    }

    fn batch_json_request(schema_version: u8) -> Vec<u8> {
        let manifests = [
            synthetic_demo_manifest_json(31).expect("seed-31 manifest"),
            synthetic_demo_manifest_json(23).expect("seed-23 manifest"),
        ];
        serde_json::to_vec(&json!({
            "jobs": manifests.map(|manifest_json| {
                let (north_deck_id, south_deck_id) =
                    manifest_deck_ids(&manifest_json).expect("manifest deck identities");
                let north_policy =
                    policy_for_deck(&manifest_json, north_deck_id.as_str()).expect("north policy");
                let south_policy =
                    policy_for_deck(&manifest_json, south_deck_id.as_str()).expect("south policy");
                json!({
                    "manifestJson": manifest_json,
                    "northDeckId": north_deck_id,
                    "northPolicy": north_policy,
                    "southDeckId": south_deck_id,
                    "southPolicy": south_policy,
                })
            }),
            "schemaVersion": schema_version,
            "workers": 2,
        }))
        .expect("batch-json request")
    }

    #[test]
    fn batch_json_should_repeat_deterministically_and_preserve_order() {
        let request = batch_json_request(1);
        let first = run_batch_json(&request).expect("first batch");
        let second = run_batch_json(&request).expect("second batch");

        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|result| result.job_index)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(
            canonical_json(&serde_json::to_value(first).expect("result JSON"))
                .expect("canonical result"),
            canonical_json(&serde_json::to_value(second).expect("result JSON"))
                .expect("canonical result")
        );
    }

    #[test]
    fn batch_json_should_reject_malformed_and_unknown_input() {
        assert!(run_batch_json(b"{").is_err());
        assert!(run_batch_json(br#"{"jobs":[],"jobs":[],"schemaVersion":1,"workers":1}"#).is_err());
        assert!(
            run_batch_json(br#"{"jobs":[],"schemaVersion":1,"unknown":true,"workers":1}"#).is_err()
        );
    }

    #[test]
    fn batch_json_should_reject_oversized_input_before_parsing() {
        assert!(validate_batch_json_size(MAX_BATCH_JSON_BYTES + 1).is_err());
    }

    #[test]
    fn batch_json_should_reject_unsupported_schema() {
        assert_eq!(
            run_batch_json(&batch_json_request(2))
                .expect_err("unsupported schema")
                .to_string(),
            "batch-json schemaVersion must be 1"
        );
    }

    #[test]
    fn batch_json_should_reject_wrong_deck_binding() {
        let request = batch_json_request(1);
        let mut value: serde_json::Value = serde_json::from_slice(&request).expect("request value");
        let south_deck_id = value["jobs"][0]["southDeckId"].clone();
        let south_policy = value["jobs"][0]["southPolicy"].clone();
        value["jobs"][0]["northDeckId"] = south_deck_id;
        value["jobs"][0]["northPolicy"] = south_policy;

        assert!(
            run_batch_json(&serde_json::to_vec(&value).expect("wrong binding request")).is_err()
        );
    }
}
