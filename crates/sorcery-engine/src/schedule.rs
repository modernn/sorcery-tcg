//! Predeclared seat-swapped seed-block schedules with caps and failure policy.

use std::error::Error;
use std::fmt;

use serde::Serialize;
use serde_json::{Value, json};

use crate::batch::{BatchClassification, BatchJob};
use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::gauntlet::{
    DeckOutcomeCounts, GauntletError, GauntletGameResult, GauntletOrientation, GauntletPair,
    GauntletReport, OutcomeCounts, SeatOutcomeCounts, run_gauntlet,
};
use crate::policy::{BASELINE_POLICY_DECK_ID, PolicySnapshot, baseline_policy_snapshot};
use crate::synthetic::synthetic_demo_manifest_json;

/// Largest number of seed blocks in one schedule.
pub const MAX_SCHEDULE_BLOCKS: usize = 16;
/// Largest times a block's seed list may repeat.
pub const MAX_BLOCK_WEIGHT: u32 = 8;
/// Largest planned pair count after expansion and caps.
pub const MAX_SCHEDULE_PAIRS: usize = 128;

/// One predeclared seed block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeedBlock<'a> {
    /// Stable block identity used in the schedule hash.
    pub id: &'a str,
    /// Declared seeds in expansion order.
    pub seeds: &'a [u32],
    /// How many times to concatenate `seeds` before the block cap.
    pub weight: u32,
    /// Maximum pairs taken from this block after weighting.
    pub max_pairs: usize,
}

/// What to do when a seat-swapped pair fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailurePolicy {
    /// Fail the whole schedule and return no report.
    Abort,
    /// Keep completed pairs and stop scheduling later blocks.
    StopAfterPairFailure,
}

/// Whether the schedule ran every planned pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleStatus {
    /// Every planned pair finished.
    Completed,
    /// A later pair failed under [`FailurePolicy::StopAfterPairFailure`].
    Stopped,
}

/// Immutable result of one expanded schedule.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleReport {
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Seeds whose pairs finished.
    pub completed_seeds: Vec<u32>,
    /// Failed seed when the schedule stopped, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_seed: Option<u32>,
    /// Applied failure policy.
    pub failure_policy: FailurePolicy,
    /// Number of finished games across completed pairs.
    pub game_count: usize,
    /// Aggregated finished pairs. Skipped from canonical JSON because it carries a float mean.
    #[serde(skip)]
    pub gauntlet: GauntletReport,
    /// Seeds after weighting and caps, including any failed seed.
    pub planned_seeds: Vec<u32>,
    /// Schedule schema.
    pub schema_version: u8,
    /// Identity of the declared blocks, caps, and policy.
    pub schedule_id: IdentityHash,
    /// Finished or stopped after a later pair failed.
    pub status: ScheduleStatus,
}

/// Expanding or running a schedule failed.
#[derive(Debug)]
pub enum ScheduleError {
    /// A declared bound or block was invalid.
    Invalid(&'static str),
    /// Canonical hashing failed.
    Canonical(CanonicalError),
    /// JSON could not be decoded while swapping a manifest.
    Json(serde_json::Error),
    /// A pair failed under [`FailurePolicy::Abort`].
    Gauntlet(GauntletError),
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => formatter.write_str(message),
            Self::Canonical(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Gauntlet(error) => error.fmt(formatter),
        }
    }
}

impl Error for ScheduleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Gauntlet(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<CanonicalError> for ScheduleError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for ScheduleError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<GauntletError> for ScheduleError {
    fn from(error: GauntletError) -> Self {
        Self::Gauntlet(error)
    }
}

/// Expands weighted, capped seed blocks into one deterministic planned list.
///
/// # Errors
///
/// Returns [`ScheduleError::Invalid`] when a block, weight, or cap is out of range.
pub fn expand_seed_blocks(
    blocks: &[SeedBlock<'_>],
    max_pairs: usize,
) -> Result<Vec<u32>, ScheduleError> {
    if blocks.is_empty() || blocks.len() > MAX_SCHEDULE_BLOCKS {
        return Err(ScheduleError::Invalid(
            "schedule must contain 1-16 seed blocks",
        ));
    }
    if max_pairs == 0 || max_pairs > MAX_SCHEDULE_PAIRS {
        return Err(ScheduleError::Invalid("schedule maxPairs must be 1-128"));
    }
    let mut planned = Vec::new();
    for block in blocks {
        if block.id.trim().is_empty()
            || block.seeds.is_empty()
            || block.weight == 0
            || block.weight > MAX_BLOCK_WEIGHT
            || block.max_pairs == 0
            || block.max_pairs > MAX_SCHEDULE_PAIRS
        {
            return Err(ScheduleError::Invalid(
                "seed block id, seeds, weight, and maxPairs are out of range",
            ));
        }
        let mut expanded = Vec::new();
        for _ in 0..block.weight {
            expanded.extend_from_slice(block.seeds);
        }
        expanded.truncate(block.max_pairs);
        planned.extend(expanded);
        if planned.len() >= max_pairs {
            planned.truncate(max_pairs);
            break;
        }
    }
    if planned.is_empty() {
        return Err(ScheduleError::Invalid("schedule planned no pairs"));
    }
    Ok(planned)
}

/// Runs a synthetic seat-swapped schedule for the expanded seed list.
///
/// # Errors
///
/// Returns [`ScheduleError`] when expansion, pairing, or an aborted pair fails.
pub fn run_synthetic_schedule(
    blocks: &[SeedBlock<'_>],
    max_pairs: usize,
    requested_workers: usize,
    failure_policy: FailurePolicy,
) -> Result<ScheduleReport, ScheduleError> {
    let planned_seeds = expand_seed_blocks(blocks, max_pairs)?;
    let schedule_id = schedule_identity(blocks, max_pairs, failure_policy)?;
    let owned = planned_seeds
        .iter()
        .map(|&seed| {
            let north = synthetic_demo_manifest_json(seed)?;
            let south = swap_manifest_decks(&north)?;
            Ok::<_, ScheduleError>((north, south))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let policy = policy_from_manifest(&owned[0].0)?;
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID)
        .map_err(|_| ScheduleError::Invalid("baseline policy deckId is invalid"))?;
    let pairs = planned_seeds
        .iter()
        .copied()
        .zip(owned.iter())
        .map(|(seed, (north, south))| synthetic_pair(seed, north, south, &policy, &deck_id))
        .collect::<Vec<_>>();
    run_declared_pairs(&pairs, requested_workers, failure_policy, schedule_id)
}

/// Runs already-built seat-swapped pairs under the declared failure policy.
///
/// # Errors
///
/// Returns [`ScheduleError`] when a pair fails under [`FailurePolicy::Abort`] or
/// no pair completes.
pub fn run_declared_pairs(
    pairs: &[GauntletPair<'_>],
    requested_workers: usize,
    failure_policy: FailurePolicy,
    schedule_id: IdentityHash,
) -> Result<ScheduleReport, ScheduleError> {
    if pairs.is_empty() || pairs.len() > MAX_SCHEDULE_PAIRS {
        return Err(ScheduleError::Invalid(
            "schedule must contain 1-128 seat-swapped pairs",
        ));
    }
    let planned_seeds = pairs.iter().map(|pair| pair.seed).collect::<Vec<_>>();
    let mut completed = Vec::new();
    let mut failed_seed = None;
    for pair in pairs {
        match run_gauntlet(&[*pair], requested_workers) {
            Ok(report) => completed.push(report),
            Err(error) => match failure_policy {
                FailurePolicy::Abort => return Err(error.into()),
                FailurePolicy::StopAfterPairFailure => {
                    failed_seed = Some(pair.seed);
                    break;
                }
            },
        }
    }
    if completed.is_empty() {
        return Err(ScheduleError::Invalid(
            "schedule completed no seat-swapped pairs",
        ));
    }
    Ok(ScheduleReport {
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        completed_seeds: planned_seeds
            .iter()
            .copied()
            .take(completed.len())
            .collect(),
        failed_seed,
        failure_policy,
        game_count: completed.iter().map(|report| report.game_count).sum(),
        gauntlet: merge_gauntlet_reports(&completed)?,
        planned_seeds,
        schema_version: 1,
        schedule_id,
        status: if failed_seed.is_none() {
            ScheduleStatus::Completed
        } else {
            ScheduleStatus::Stopped
        },
    })
}

fn policy_from_manifest(manifest_json: &str) -> Result<PolicySnapshot, ScheduleError> {
    let game = crate::game::Game::from_manifest_json(manifest_json)
        .map_err(|_| ScheduleError::Invalid("schedule could not parse the first manifest"))?;
    baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
        .map_err(|_| ScheduleError::Invalid("schedule could not bind the baseline policy"))
}

fn schedule_identity(
    blocks: &[SeedBlock<'_>],
    max_pairs: usize,
    failure_policy: FailurePolicy,
) -> Result<IdentityHash, ScheduleError> {
    identity_hash(&json!({
        "blocks": blocks
            .iter()
            .map(|block| {
                json!({
                    "id": block.id,
                    "maxPairs": block.max_pairs,
                    "seeds": block.seeds,
                    "weight": block.weight,
                })
            })
            .collect::<Vec<_>>(),
        "failurePolicy": failure_policy,
        "maxPairs": max_pairs,
        "schemaVersion": 1,
    }))
    .map_err(ScheduleError::from)
}

fn swap_manifest_decks(manifest_json: &str) -> Result<String, ScheduleError> {
    let mut manifest: Value = serde_json::from_str(manifest_json)?;
    let object = manifest.as_object_mut().ok_or(ScheduleError::Invalid(
        "schedule manifest must be an object",
    ))?;
    object.remove("manifestId");
    let decks = object
        .get_mut("decks")
        .and_then(Value::as_object_mut)
        .ok_or(ScheduleError::Invalid(
            "schedule manifest must declare both decks",
        ))?;
    let north = decks.remove("north").ok_or(ScheduleError::Invalid(
        "schedule manifest must declare both decks",
    ))?;
    let south = decks.remove("south").ok_or(ScheduleError::Invalid(
        "schedule manifest must declare both decks",
    ))?;
    decks.insert("north".to_owned(), south);
    decks.insert("south".to_owned(), north);
    let manifest_id = identity_hash(&manifest)?;
    manifest["manifestId"] = json!(manifest_id);
    crate::canonical::canonical_json(&manifest).map_err(ScheduleError::from)
}

fn synthetic_pair<'a>(
    seed: u32,
    north: &'a str,
    south: &'a str,
    policy: &'a PolicySnapshot,
    deck_id: &'a IdentityHash,
) -> GauntletPair<'a> {
    GauntletPair {
        seed,
        orientations: [
            GauntletOrientation {
                job: BatchJob {
                    manifest_json: north,
                    north_deck_id: deck_id,
                    north_policy: policy,
                    south_deck_id: deck_id,
                    south_policy: policy,
                },
                north_deck_id: "north",
                south_deck_id: "south",
            },
            GauntletOrientation {
                job: BatchJob {
                    manifest_json: south,
                    north_deck_id: deck_id,
                    north_policy: policy,
                    south_deck_id: deck_id,
                    south_policy: policy,
                },
                north_deck_id: "south",
                south_deck_id: "north",
            },
        ],
    }
}

fn merge_gauntlet_reports(reports: &[GauntletReport]) -> Result<GauntletReport, ScheduleError> {
    let mut games = Vec::new();
    let mut by_deck = std::collections::BTreeMap::<String, DeckOutcomeCounts>::new();
    let mut by_seat = SeatOutcomeCounts::default();
    let mut seeds = Vec::new();
    let mut job_index = 0_usize;
    for report in reports {
        seeds.extend_from_slice(&report.seeds);
        add_seat(&mut by_seat.north, report.by_seat.north);
        add_seat(&mut by_seat.south, report.by_seat.south);
        for (deck_id, counts) in &report.by_deck {
            let entry = by_deck.entry(deck_id.clone()).or_default();
            add_deck(entry, counts);
        }
        for game in &report.games {
            let mut remapped = game.clone();
            remapped.result.job_index = job_index;
            job_index = job_index
                .checked_add(1)
                .ok_or(ScheduleError::Invalid("schedule job index overflowed"))?;
            games.push(remapped);
        }
    }
    average_report(by_deck, by_seat, games, seeds)
}

fn add_seat(total: &mut OutcomeCounts, add: OutcomeCounts) {
    total.draws += add.draws;
    total.games += add.games;
    total.losses += add.losses;
    total.wins += add.wins;
}

fn add_deck(total: &mut DeckOutcomeCounts, add: &DeckOutcomeCounts) {
    add_seat(&mut total.as_north, add.as_north);
    add_seat(&mut total.as_south, add.as_south);
    total.draws += add.draws;
    total.games += add.games;
    total.losses += add.losses;
    total.wins += add.wins;
}

fn average_report(
    by_deck: std::collections::BTreeMap<String, DeckOutcomeCounts>,
    by_seat: SeatOutcomeCounts,
    games: Vec<GauntletGameResult>,
    seeds: Vec<u32>,
) -> Result<GauntletReport, ScheduleError> {
    let total_turns = games
        .iter()
        .try_fold(0_u64, |total, game| {
            total.checked_add(game.result.report.turn_count)
        })
        .ok_or(ScheduleError::Invalid("schedule turn total overflowed"))?;
    let average_turns = f64::from(
        u32::try_from(total_turns)
            .map_err(|_| ScheduleError::Invalid("schedule turn total overflowed"))?,
    ) / f64::from(
        u32::try_from(games.len())
            .map_err(|_| ScheduleError::Invalid("schedule game count overflowed"))?,
    );
    Ok(GauntletReport {
        average_turns,
        by_deck,
        by_seat,
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        game_count: games.len(),
        games,
        seeds,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FailurePolicy, ScheduleStatus, SeedBlock, expand_seed_blocks, run_declared_pairs,
        run_synthetic_schedule,
    };
    use crate::batch::BatchJob;
    use crate::canonical::IdentityHash;
    use crate::gauntlet::{GauntletOrientation, GauntletPair};
    use crate::policy::{BASELINE_POLICY_DECK_ID, baseline_policy_snapshot};
    use crate::synthetic::synthetic_demo_manifest_json;

    const SCHEDULE_ID: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn expand_applies_weight_then_block_and_global_caps() {
        let early = SeedBlock {
            id: "early",
            seeds: &[31, 23],
            weight: 2,
            max_pairs: 3,
        };
        let late = SeedBlock {
            id: "late",
            seeds: &[7],
            weight: 1,
            max_pairs: 1,
        };
        assert_eq!(
            expand_seed_blocks(&[early, late], 3).expect("expanded"),
            [31, 23, 31]
        );
        assert!(expand_seed_blocks(&[early], 0).is_err());
        assert!(expand_seed_blocks(&[SeedBlock { weight: 0, ..early }], 1).is_err());
    }

    #[test]
    fn synthetic_schedule_runs_the_seed_31_seat_swap() {
        let report = run_synthetic_schedule(
            &[SeedBlock {
                id: "demo",
                seeds: &[31],
                weight: 1,
                max_pairs: 1,
            }],
            1,
            2,
            FailurePolicy::Abort,
        )
        .expect("seed-31 schedule");
        assert_eq!(report.status, ScheduleStatus::Completed);
        assert_eq!(report.planned_seeds, [31]);
        assert_eq!(report.completed_seeds, [31]);
        assert_eq!(report.failed_seed, None);
        assert_eq!(report.gauntlet.game_count, 2);
        assert_eq!(report.gauntlet.seeds, [31]);
        assert_eq!(report.gauntlet.by_seat.north.games, 2);
        assert_eq!(report.gauntlet.by_seat.south.games, 2);
        assert!(
            report
                .gauntlet
                .games
                .iter()
                .all(|game| game.result.report.replay_verified)
        );
    }

    #[test]
    fn stop_policy_keeps_the_finished_pair() {
        let north = synthetic_demo_manifest_json(31).expect("manifest");
        let south = super::swap_manifest_decks(&north).expect("swapped");
        let game = crate::game::Game::from_manifest_json(&north).expect("game");
        let policy =
            baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
                .expect("policy");
        let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("deck");
        let good = super::synthetic_pair(31, &north, &south, &policy, &deck_id);
        let bad = GauntletPair {
            seed: 23,
            orientations: [
                GauntletOrientation {
                    job: BatchJob {
                        manifest_json: "{}",
                        ..good.orientations[0].job
                    },
                    ..good.orientations[0]
                },
                good.orientations[1],
            ],
        };
        assert!(
            run_declared_pairs(
                &[bad],
                1,
                FailurePolicy::Abort,
                IdentityHash::parse(SCHEDULE_ID).expect("id"),
            )
            .is_err()
        );
        let stopped = run_declared_pairs(
            &[good, bad],
            2,
            FailurePolicy::StopAfterPairFailure,
            IdentityHash::parse(SCHEDULE_ID).expect("id"),
        )
        .expect("stopped schedule");
        assert_eq!(stopped.status, ScheduleStatus::Stopped);
        assert_eq!(stopped.completed_seeds, [31]);
        assert_eq!(stopped.failed_seed, Some(23));
        assert_eq!(stopped.planned_seeds, [31, 23]);
        assert_eq!(stopped.gauntlet.game_count, 2);
    }
}
