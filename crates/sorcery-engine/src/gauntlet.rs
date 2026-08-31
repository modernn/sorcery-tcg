//! Deterministic seat-swapped gauntlet aggregation over native batches.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::batch::{
    BatchClassification, BatchError, BatchJob, BatchResult, MAX_BATCH_JOBS, run_batch,
};
use crate::contract::Seat;
use crate::game::GameOutcome;

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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeckOutcomeCounts {
    /// Overall counts.
    pub total: OutcomeCounts,
    /// Counts while occupying North.
    pub as_north: OutcomeCounts,
    /// Counts while occupying South.
    pub as_south: OutcomeCounts,
}

/// One authoritative gauntlet game with deck and seed metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GauntletGameResult {
    /// Native authoritative result.
    pub result: BatchResult,
    /// Stable North deck identity.
    pub north_deck_id: String,
    /// Declared seed.
    pub seed: u32,
    /// Stable South deck identity.
    pub south_deck_id: String,
}

/// Complete deterministic gauntlet report.
#[derive(Clone, Debug, PartialEq)]
pub struct GauntletReport {
    /// Arithmetic mean of final turn numbers.
    pub average_turns: f64,
    /// Canonically ordered per-deck counts.
    pub by_deck: BTreeMap<String, DeckOutcomeCounts>,
    /// North then South counts.
    pub by_seat: [OutcomeCounts; 2],
    /// Ranked/public result classification.
    pub classification: BatchClassification,
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
    let mut by_seat = [OutcomeCounts::default(); 2];
    let mut games = Vec::with_capacity(results.len());
    for result in results {
        let pair = &pairs[result.job_index / 2];
        let orientation = &pair.orientations[result.job_index % 2];
        for (seat, deck_id) in [
            (Seat::North, orientation.north_deck_id),
            (Seat::South, orientation.south_deck_id),
        ] {
            let outcome = perspective(result.outcome, seat);
            record(&mut by_seat[seat_index(seat)], outcome);
            let deck = by_deck.entry(deck_id.to_owned()).or_default();
            record(&mut deck.total, outcome);
            record(
                match seat {
                    Seat::North => &mut deck.as_north,
                    Seat::South => &mut deck.as_south,
                },
                outcome,
            );
        }
        games.push(GauntletGameResult {
            result,
            north_deck_id: orientation.north_deck_id.to_owned(),
            seed: pair.seed,
            south_deck_id: orientation.south_deck_id.to_owned(),
        });
    }
    let total_turns = games
        .iter()
        .try_fold(0_u64, |total, game| {
            total.checked_add(game.result.turn_count)
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
        classification: BatchClassification::UnrankedPartialRules,
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

fn perspective(outcome: GameOutcome, seat: Seat) -> Perspective {
    match outcome {
        GameOutcome::Draw => Perspective::Draw,
        GameOutcome::Win { winner, .. } if winner == seat => Perspective::Win,
        GameOutcome::Win { .. } => Perspective::Loss,
    }
}

const fn seat_index(seat: Seat) -> usize {
    match seat {
        Seat::North => 0,
        Seat::South => 1,
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

fn validate_pairs(pairs: &[GauntletPair<'_>]) -> Result<(), GauntletError> {
    for pair in pairs {
        let [first, second] = pair.orientations;
        if first.north_deck_id.trim().is_empty()
            || first.south_deck_id.trim().is_empty()
            || first.north_deck_id == first.south_deck_id
            || second.north_deck_id != first.south_deck_id
            || second.south_deck_id != first.north_deck_id
        {
            return Err(GauntletError::Invalid(
                "gauntlet requires two distinct decks swapped across seats",
            ));
        }
        for orientation in pair.orientations {
            let manifest: serde_json::Value = serde_json::from_str(orientation.job.manifest_json)?;
            if manifest.get("seed").and_then(serde_json::Value::as_u64)
                != Some(u64::from(pair.seed))
            {
                return Err(GauntletError::Invalid(
                    "gauntlet seed does not match its manifest",
                ));
            }
        }
    }
    Ok(())
}
