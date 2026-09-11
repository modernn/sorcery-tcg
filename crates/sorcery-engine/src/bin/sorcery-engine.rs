use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sorcery_engine::batch::{
    BatchJob, DeterministicGameReport, GameBatchResult, MAX_BATCH_BYTES, MAX_BATCH_JOBS,
    MAX_BATCH_WORKERS, default_batch_workers, run_game_batch, run_game_batch_to_dir,
};
use sorcery_engine::canonical::{
    IdentityHash, canonical_json, identity_hash, parse_json_without_duplicate_keys,
};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, FormatContext, OfficialCardMapping,
    validate_deck,
};
use sorcery_engine::game_record::{
    record_synthetic_demo, replay_game_artifacts, write_game_artifacts,
};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::schedule::{FailurePolicy, SeedBlock, run_synthetic_schedule};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const SYNTHETIC_DECK_ID: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MAX_BATCH_JSON_BYTES: usize = MAX_BATCH_BYTES * 2 + 1024 * 1024;

type CliResult<T> = Result<T, Box<dyn Error>>;

enum Command {
    Demo {
        seed: u32,
        artifacts_dir: Option<String>,
    },
    Record {
        seed: u32,
        artifacts_dir: Option<String>,
    },
    Batch {
        workers: usize,
        seeds: Vec<u32>,
        artifacts_dir: Option<String>,
    },
    BatchJson,
    Schedule {
        workers: usize,
        seeds: Vec<u32>,
        artifacts_dir: Option<String>,
    },
    Replay {
        artifacts_dir: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BatchJsonRequest {
    schema_version: u8,
    workers: usize,
    artifacts_dir: Option<String>,
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
        Command::Demo {
            seed,
            artifacts_dir,
        } => {
            if let Some(dir) = artifacts_dir {
                let record = record_synthetic_demo(seed)?;
                write_game_artifacts(Path::new(&dir), &record)?;
                write_canonical_json(&compact_report(&record))
            } else {
                let mut results = run_synthetic_batch(&[seed], 1, None)?;
                let report = results
                    .pop()
                    .ok_or_else(|| io::Error::other("demo produced no result"))?
                    .report;
                write_canonical_json(&report)
            }
        }
        Command::Record {
            seed,
            artifacts_dir,
        } => {
            let record = record_synthetic_demo(seed)?;
            if let Some(dir) = artifacts_dir {
                write_game_artifacts(Path::new(&dir), &record)?;
            }
            write_canonical_json(&record)
        }
        Command::Batch {
            workers,
            seeds,
            artifacts_dir,
        } => write_canonical_json(&run_synthetic_batch(
            &seeds,
            workers,
            artifacts_dir.as_deref(),
        )?),
        Command::BatchJson => {
            let input = read_batch_json_stdin()?;
            write_canonical_json(&run_batch_json(&input)?)
        }
        Command::Schedule {
            workers,
            seeds,
            artifacts_dir,
        } => write_canonical_json(&run_synthetic_schedule(
            &[SeedBlock {
                id: "cli",
                seeds: &seeds,
                weight: 1,
                max_pairs: seeds.len(),
            }],
            seeds.len(),
            workers,
            FailurePolicy::Abort,
            artifacts_dir.as_deref().map(Path::new),
        )?),
        Command::Replay { artifacts_dir } => {
            write_canonical_json(&replay_game_artifacts(Path::new(&artifacts_dir))?)
        }
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> CliResult<Command> {
    let mut args = args;
    match args.next().as_deref() {
        Some("demo") => {
            let seed = args.next().map_or(Ok(1), |value| parse_seed(&value))?;
            let artifacts_dir = args.next();
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine demo [seed] [dir]").into());
            }
            Ok(Command::Demo {
                seed,
                artifacts_dir,
            })
        }
        Some("record") => {
            let seed = args.next().map_or(Ok(1), |value| parse_seed(&value))?;
            let artifacts_dir = args.next();
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine record [seed] [dir]").into());
            }
            Ok(Command::Record {
                seed,
                artifacts_dir,
            })
        }
        Some("batch") => parse_batch_args(args),
        Some("schedule") => parse_schedule_args(args),
        Some("replay") => {
            let artifacts_dir = args
                .next()
                .ok_or_else(|| io::Error::other("usage: sorcery-engine replay dir"))?;
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine replay dir").into());
            }
            Ok(Command::Replay { artifacts_dir })
        }
        Some("batch-json") => {
            if args.next().is_some() {
                return Err(io::Error::other("usage: sorcery-engine batch-json").into());
            }
            Ok(Command::BatchJson)
        }
        _ => Err(io::Error::other(
            "usage: sorcery-engine demo [seed] [dir] | record [seed] [dir] | batch [--out dir] [workers] [seeds...] | schedule [--out dir] [workers] [seeds...] | replay dir | batch-json",
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
    let artifacts_dir = request.artifacts_dir;
    let workers = request.workers;
    let validated = request
        .jobs
        .into_iter()
        .map(|job| {
            let (north_manifest_deck_id, south_manifest_deck_id) =
                manifest_deck_ids(&job.manifest_json)?;
            if job.north_deck_id != north_manifest_deck_id
                || job.south_deck_id != south_manifest_deck_id
            {
                return Err(io::Error::other(
                    "batch-json deck IDs do not match manifest deck composition",
                )
                .into());
            }
            Ok(ValidatedBatchJsonJob {
                manifest_json: job.manifest_json,
                north_deck_id: job.north_deck_id,
                north_policy: parse_policy_snapshot(&canonical_json(&job.north_policy)?)?,
                south_deck_id: job.south_deck_id,
                south_policy: parse_policy_snapshot(&canonical_json(&job.south_policy)?)?,
            })
        })
        .collect::<CliResult<Vec<_>>>()?;
    let jobs = validated
        .iter()
        .map(|job| BatchJob {
            manifest_json: &job.manifest_json,
            north_deck_id: &job.north_deck_id,
            north_policy: &job.north_policy,
            south_deck_id: &job.south_deck_id,
            south_policy: &job.south_policy,
        })
        .collect::<Vec<_>>();
    if let Some(dir) = artifacts_dir {
        Ok(run_game_batch_to_dir(&jobs, workers, Path::new(&dir))?)
    } else {
        Ok(run_game_batch(&jobs, workers)?)
    }
}

fn parse_batch_args(args: impl Iterator<Item = String>) -> CliResult<Command> {
    let mut artifacts_dir = None;
    let mut rest = Vec::new();
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--out" {
            let dir = args
                .next()
                .ok_or_else(|| io::Error::other("usage: sorcery-engine batch --out dir"))?;
            artifacts_dir = Some(dir);
            continue;
        }
        rest.push(arg);
    }
    let mut rest = rest.into_iter();
    let workers = rest.next().map_or_else(
        || Ok(default_batch_workers()),
        |value| parse_workers(&value),
    )?;
    let mut seeds = rest
        .map(|value| parse_seed(&value))
        .collect::<Result<Vec<_>, _>>()?;
    if seeds.is_empty() {
        seeds.push(1);
    }
    if seeds.len() > MAX_BATCH_JOBS {
        return Err(io::Error::other("batch must contain 1-256 seeds").into());
    }
    Ok(Command::Batch {
        workers,
        seeds,
        artifacts_dir,
    })
}

fn parse_schedule_args(args: impl Iterator<Item = String>) -> CliResult<Command> {
    let mut artifacts_dir = None;
    let mut rest = Vec::new();
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--out" {
            let dir = args
                .next()
                .ok_or_else(|| io::Error::other("usage: sorcery-engine schedule --out dir"))?;
            artifacts_dir = Some(dir);
            continue;
        }
        rest.push(arg);
    }
    let mut rest = rest.into_iter();
    let workers = rest.next().map_or_else(
        || Ok(default_batch_workers()),
        |value| parse_workers(&value),
    )?;
    let mut seeds = rest
        .map(|value| parse_seed(&value))
        .collect::<Result<Vec<_>, _>>()?;
    if seeds.is_empty() {
        seeds.push(1);
    }
    if seeds.len() > MAX_BATCH_JOBS / 2 {
        return Err(io::Error::other("schedule must contain 1-128 seeds").into());
    }
    Ok(Command::Schedule {
        workers,
        seeds,
        artifacts_dir,
    })
}

fn compact_report(record: &sorcery_engine::game_record::GameRecord) -> DeterministicGameReport {
    DeterministicGameReport {
        accepted_action_count: record.accepted_action_count,
        classification: record.classification,
        final_state_hash: record.final_state_hash.clone(),
        fight_count: record.fight_count,
        replay_verified: record.replay_verified,
        terminal: record.terminal,
        transcript_hash: record.transcript_hash.clone(),
        turn_count: record.turn_count,
    }
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

fn run_synthetic_batch(
    seeds: &[u32],
    workers: usize,
    artifacts_dir: Option<&str>,
) -> CliResult<Vec<GameBatchResult>> {
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
    if let Some(dir) = artifacts_dir {
        Ok(run_game_batch_to_dir(&jobs, workers, Path::new(dir))?)
    } else {
        Ok(run_game_batch(&jobs, workers)?)
    }
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
    fn parse_args_should_require_replay_dir() {
        let command = parse_args(["replay".to_owned(), "games/31".to_owned()].into_iter())
            .expect("valid replay");
        let Command::Replay { artifacts_dir } = command else {
            panic!("expected replay command");
        };
        assert_eq!(artifacts_dir, "games/31");
        assert!(parse_args(["replay".to_owned()].into_iter()).is_err());
    }

    #[test]
    fn parse_args_should_default_record_seed() {
        let command = parse_args(["record".to_owned()].into_iter()).expect("valid record");
        let Command::Record {
            seed,
            artifacts_dir,
        } = command
        else {
            panic!("expected record command");
        };

        assert_eq!((seed, artifacts_dir), (1, None));
    }

    #[test]
    fn parse_args_should_default_schedule_seed() {
        let command = parse_args(["schedule".to_owned()].into_iter()).expect("valid schedule");
        let Command::Schedule {
            seeds,
            workers,
            artifacts_dir,
        } = command
        else {
            panic!("expected schedule command");
        };

        assert_eq!(
            (seeds, (1..=8).contains(&workers), artifacts_dir),
            (vec![1], true, None)
        );
    }

    #[test]
    fn parse_args_should_take_demo_and_batch_artifact_dirs() {
        let demo =
            parse_args(["demo".to_owned(), "31".to_owned(), "games/31".to_owned()].into_iter())
                .expect("valid demo");
        let Command::Demo {
            seed,
            artifacts_dir,
        } = demo
        else {
            panic!("expected demo command");
        };
        assert_eq!((seed, artifacts_dir.as_deref()), (31, Some("games/31")));

        let batch = parse_args(
            [
                "batch".to_owned(),
                "--out".to_owned(),
                "games".to_owned(),
                "2".to_owned(),
                "31".to_owned(),
            ]
            .into_iter(),
        )
        .expect("valid batch");
        let Command::Batch {
            seeds,
            workers,
            artifacts_dir,
        } = batch
        else {
            panic!("expected batch command");
        };
        assert_eq!(
            (workers, seeds, artifacts_dir.as_deref()),
            (2, vec![31], Some("games"))
        );

        let schedule = parse_args(
            [
                "schedule".to_owned(),
                "--out".to_owned(),
                "games".to_owned(),
                "2".to_owned(),
                "31".to_owned(),
            ]
            .into_iter(),
        )
        .expect("valid schedule");
        let Command::Schedule {
            seeds,
            workers,
            artifacts_dir,
        } = schedule
        else {
            panic!("expected schedule command");
        };
        assert_eq!(
            (workers, seeds, artifacts_dir.as_deref()),
            (2, vec![31], Some("games"))
        );
    }

    #[test]
    fn parse_args_should_apply_batch_defaults() {
        let command = parse_args(["batch".to_owned()].into_iter()).expect("valid defaults");
        let Command::Batch {
            seeds,
            workers,
            artifacts_dir,
        } = command
        else {
            panic!("expected batch command");
        };

        assert_eq!(
            (seeds, (1..=8).contains(&workers), artifacts_dir),
            (vec![1], true, None)
        );
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
