use std::error::Error;
use std::io::{self, Write};

use serde::Serialize;
use serde_json::{Value, json};
use sorcery_engine::batch::{
    BatchJob, GameBatchResult, MAX_BATCH_JOBS, MAX_BATCH_WORKERS, default_batch_workers,
    run_game_batch,
};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const SYNTHETIC_DECK_ID: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

type CliResult<T> = Result<T, Box<dyn Error>>;

enum Command {
    Demo { seed: u32 },
    Batch { workers: usize, seeds: Vec<u32> },
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
        _ => Err(io::Error::other(
            "usage: sorcery-engine demo [seed] | batch [workers] [seeds...]",
        )
        .into()),
    }
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
    let manifest: Value = serde_json::from_str(manifest_json)?;
    let mut body = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": SYNTHETIC_DECK_ID,
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
    use super::{Command, parse_args};

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
}
