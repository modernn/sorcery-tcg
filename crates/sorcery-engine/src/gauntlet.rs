//! Deterministic seat-swapped gauntlet aggregation over native batches.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::Serialize;

use crate::batch::{
    BatchClassification, BatchError, BatchJob, FinishedTerminal, GameBatchResult, MAX_BATCH_JOBS,
    run_batch,
};
use crate::contract::Seat;

/// One deck orientation for a gauntlet seed.
#[derive(Clone, Copy, Debug)]
pub struct GauntletOrientation<'a> {
    /// Complete native game job.
    pub job: BatchJob<'a>,
    /// Stable deck identity occupying North.
    pub north_deck_id: &'a str,
    /// Stable deck identity occupying South.
    pub south_deck_id: &'a str,
}

/// Both seat orientations for one seed and deck pairing.
#[derive(Clone, Copy, Debug)]
pub struct GauntletPair<'a> {
    /// Declared manifest seed.
    pub seed: u32,
    /// Same two decks in opposite seats.
    pub orientations: [GauntletOrientation<'a>; 2],
}

/// Win/draw/loss counts from one perspective.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct OutcomeCounts {
    /// Draws.
    pub draws: u64,
    /// Counted games.
    pub games: u64,
    /// Losses.
    pub losses: u64,
    /// Wins.
    pub wins: u64,
}

/// Aggregate results for one deck overall and by seat.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckOutcomeCounts {
    /// Counts while occupying North.
    pub as_north: OutcomeCounts,
    /// Counts while occupying South.
    pub as_south: OutcomeCounts,
    /// Overall draws.
    pub draws: u64,
    /// Overall counted games.
    pub games: u64,
    /// Overall losses.
    pub losses: u64,
    /// Overall wins.
    pub wins: u64,
}

/// One authoritative gauntlet game with deck and seed metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GauntletGameResult {
    /// Native authoritative result.
    #[serde(flatten)]
    pub result: GameBatchResult,
    /// Stable North deck identity.
    pub north_deck_id: String,
    /// Declared seed.
    pub seed: u32,
    /// Stable South deck identity.
    pub south_deck_id: String,
}

/// Seat-keyed outcome counts matching the public TypeScript contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SeatOutcomeCounts {
    /// North-seat outcomes.
    pub north: OutcomeCounts,
    /// South-seat outcomes.
    pub south: OutcomeCounts,
}

/// Complete deterministic gauntlet report.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GauntletReport {
    /// Arithmetic mean of final turn numbers.
    pub average_turns: f64,
    /// Canonically ordered per-deck counts.
    pub by_deck: BTreeMap<String, DeckOutcomeCounts>,
    /// Physical-seat outcome counts.
    pub by_seat: SeatOutcomeCounts,
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Number of games in the report.
    pub game_count: usize,
    /// Ordered authoritative game results.
    pub games: Vec<GauntletGameResult>,
    /// Input seeds in pair order.
    pub seeds: Vec<u32>,
}

/// Gauntlet input, manifest metadata, or native batch execution failed.
#[derive(Debug)]
pub enum GauntletError {
    /// A gauntlet invariant was violated.
    Invalid(&'static str),
    /// Manifest JSON could not be decoded while validating its seed.
    Json(serde_json::Error),
    /// Native batch execution failed.
    Batch(BatchError),
}

impl fmt::Display for GauntletError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => formatter.write_str(message),
            Self::Json(error) => error.fmt(formatter),
            Self::Batch(error) => error.fmt(formatter),
        }
    }
}

impl Error for GauntletError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Batch(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<serde_json::Error> for GauntletError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<BatchError> for GauntletError {
    fn from(error: BatchError) -> Self {
        Self::Batch(error)
    }
}

/// Runs exact two-seat pairs and aggregates outcomes without leaving Rust.
///
/// # Errors
///
/// Returns [`GauntletError`] for malformed pairs, seed mismatches, or batch failures.
pub fn run_gauntlet(
    pairs: &[GauntletPair<'_>],
    requested_workers: usize,
) -> Result<GauntletReport, GauntletError> {
    if pairs.is_empty() || pairs.len() > MAX_BATCH_JOBS / 2 {
        return Err(GauntletError::Invalid(
            "gauntlet must contain 1-128 seed pairs",
        ));
    }
    validate_pairs(pairs)?;
    let jobs = pairs
        .iter()
        .flat_map(|pair| pair.orientations.map(|orientation| orientation.job))
        .collect::<Vec<_>>();
    let results = run_batch(&jobs, requested_workers)?;
    let mut by_deck = BTreeMap::<String, DeckOutcomeCounts>::new();
    let mut by_seat = SeatOutcomeCounts::default();
    let mut games = Vec::with_capacity(results.len());
    for result in results {
        let pair = &pairs[result.job_index / 2];
        let orientation = &pair.orientations[result.job_index % 2];
        for (seat, deck_id) in [
            (Seat::North, orientation.north_deck_id),
            (Seat::South, orientation.south_deck_id),
        ] {
            let outcome = perspective(result.terminal, seat);
            record(
                match seat {
                    Seat::North => &mut by_seat.north,
                    Seat::South => &mut by_seat.south,
                },
                outcome,
            );
            let deck = by_deck.entry(deck_id.to_owned()).or_default();
            record_deck_total(deck, outcome);
            record(
                match seat {
                    Seat::North => &mut deck.as_north,
                    Seat::South => &mut deck.as_south,
                },
                outcome,
            );
        }
        games.push(GauntletGameResult {
            result: result.into(),
            north_deck_id: orientation.north_deck_id.to_owned(),
            seed: pair.seed,
            south_deck_id: orientation.south_deck_id.to_owned(),
        });
    }
    let total_turns = games
        .iter()
        .try_fold(0_u64, |total, game| {
            total.checked_add(game.result.report.turn_count)
        })
        .ok_or(GauntletError::Invalid("gauntlet turn total overflowed"))?;
    let average_turns = f64::from(
        u32::try_from(total_turns)
            .map_err(|_| GauntletError::Invalid("gauntlet turn total overflowed"))?,
    ) / f64::from(
        u32::try_from(games.len())
            .map_err(|_| GauntletError::Invalid("gauntlet game count overflowed"))?,
    );
    Ok(GauntletReport {
        average_turns,
        by_deck,
        by_seat,
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        game_count: games.len(),
        games,
        seeds: pairs.iter().map(|pair| pair.seed).collect(),
    })
}

#[derive(Clone, Copy)]
enum Perspective {
    Draw,
    Loss,
    Win,
}

fn perspective(terminal: FinishedTerminal, seat: Seat) -> Perspective {
    match terminal.winner() {
        None => Perspective::Draw,
        Some(winner) if winner == seat => Perspective::Win,
        Some(_) => Perspective::Loss,
    }
}

fn record(counts: &mut OutcomeCounts, outcome: Perspective) {
    counts.games += 1;
    match outcome {
        Perspective::Draw => counts.draws += 1,
        Perspective::Loss => counts.losses += 1,
        Perspective::Win => counts.wins += 1,
    }
}

fn record_deck_total(counts: &mut DeckOutcomeCounts, outcome: Perspective) {
    counts.games += 1;
    match outcome {
        Perspective::Draw => counts.draws += 1,
        Perspective::Loss => counts.losses += 1,
        Perspective::Win => counts.wins += 1,
    }
}

fn validate_pairs(pairs: &[GauntletPair<'_>]) -> Result<(), GauntletError> {
    let [reference_first, reference_second] = pairs[0].orientations;
    if reference_first.north_deck_id.trim().is_empty()
        || reference_first.south_deck_id.trim().is_empty()
        || reference_first.north_deck_id == reference_first.south_deck_id
        || reference_second.north_deck_id != reference_first.south_deck_id
        || reference_second.south_deck_id != reference_first.north_deck_id
        || reference_second.job.north_deck_id != reference_first.job.south_deck_id
        || reference_second.job.south_deck_id != reference_first.job.north_deck_id
    {
        return Err(GauntletError::Invalid(
            "gauntlet requires two distinct decks swapped across seats",
        ));
    }
    let reference_manifest = manifest_facts(reference_first.job.manifest_json)?;
    let deck_a = reference_manifest.north_deck;
    let deck_b = reference_manifest.south_deck;

    for pair in pairs {
        let [first, second] = pair.orientations;
        if first.north_deck_id != reference_first.north_deck_id
            || first.south_deck_id != reference_first.south_deck_id
            || second.north_deck_id != first.south_deck_id
            || second.south_deck_id != first.north_deck_id
            || first.job.north_deck_id != reference_first.job.north_deck_id
            || first.job.south_deck_id != reference_first.job.south_deck_id
            || second.job.north_deck_id != reference_second.job.north_deck_id
            || second.job.south_deck_id != reference_second.job.south_deck_id
        {
            return Err(GauntletError::Invalid(
                "gauntlet requires two distinct decks swapped across seats",
            ));
        }
        let first_manifest = manifest_facts(first.job.manifest_json)?;
        let second_manifest = manifest_facts(second.job.manifest_json)?;
        if first_manifest.seed != u64::from(pair.seed)
            || second_manifest.seed != u64::from(pair.seed)
        {
            return Err(GauntletError::Invalid(
                "gauntlet seed does not match its manifest",
            ));
        }
        if first_manifest.north_deck != deck_a
            || first_manifest.south_deck != deck_b
            || second_manifest.north_deck != deck_b
            || second_manifest.south_deck != deck_a
        {
            return Err(GauntletError::Invalid(
                "gauntlet manifests must contain the declared seat swap",
            ));
        }
    }
    Ok(())
}

struct ManifestFacts {
    north_deck: serde_json::Value,
    seed: u64,
    south_deck: serde_json::Value,
}

fn manifest_facts(manifest_json: &str) -> Result<ManifestFacts, GauntletError> {
    let manifest: serde_json::Value = serde_json::from_str(manifest_json)?;
    if manifest
        .get("firstSeat")
        .and_then(serde_json::Value::as_str)
        != Some("north")
    {
        return Err(GauntletError::Invalid(
            "gauntlet manifests must start with north",
        ));
    }
    let seed = manifest
        .get("seed")
        .and_then(serde_json::Value::as_u64)
        .ok_or(GauntletError::Invalid(
            "gauntlet manifest must declare an unsigned seed",
        ))?;
    let decks = manifest
        .get("decks")
        .and_then(serde_json::Value::as_object)
        .ok_or(GauntletError::Invalid(
            "gauntlet manifest must declare both decks",
        ))?;
    let north_deck = decks.get("north").cloned().ok_or(GauntletError::Invalid(
        "gauntlet manifest must declare both decks",
    ))?;
    let south_deck = decks.get("south").cloned().ok_or(GauntletError::Invalid(
        "gauntlet manifest must declare both decks",
    ))?;
    Ok(ManifestFacts {
        north_deck,
        seed,
        south_deck,
    })
}
