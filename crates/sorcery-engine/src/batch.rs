//! Ordered native rollout batches with authoritative final replay.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::thread;

use serde::Serialize;

use crate::canonical::IdentityHash;
use crate::contract::Seat;
use crate::game::{Game, GameEndReason, GameOutcome};
use crate::game_record::{game_record_from_session, validate_artifacts_dir, write_game_artifacts};
use crate::policy::PolicySnapshot;
use crate::session::{Session, SessionError};
use crate::simulator::{SimulatorError, replay_selected, run_game};

/// Maximum jobs accepted by one bounded batch.
pub const MAX_BATCH_JOBS: usize = 256;
/// Maximum native workers accepted by one batch.
pub const MAX_BATCH_WORKERS: usize = 8;
/// Maximum aggregate canonical manifest bytes accepted by one batch.
pub const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
/// Existing deterministic-agent action limit for one complete game.
pub const MAX_GAME_ACTIONS: usize = 500;

/// Public result classification while supported mechanics remain incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchClassification {
    /// Rules are partial and raw manifests lack independently verified authority binding.
    UnrankedPartialRulesUnverifiedAuthority,
}

/// One complete native rollout job.
#[derive(Clone, Copy, Debug)]
pub struct BatchJob<'a> {
    /// Canonical authoritative manifest bytes.
    pub manifest_json: &'a str,
    /// Deck identity North's policy must be bound to.
    pub north_deck_id: &'a IdentityHash,
    /// North's immutable deck-bound policy.
    pub north_policy: &'a PolicySnapshot,
    /// Deck identity South's policy must be bound to.
    pub south_deck_id: &'a IdentityHash,
    /// South's immutable deck-bound policy.
    pub south_policy: &'a PolicySnapshot,
}

/// The status of a finished public terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FinishedStatus {
    /// The game finished.
    Finished,
}

/// The public result marker for a draw.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DrawResult {
    /// Neither seat won.
    Draw,
}

/// Why a finished game was drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawReason {
    /// Both Avatars were defeated by one simultaneous damage batch.
    SimultaneousAvatarDefeat,
    /// Both seats otherwise lost simultaneously.
    SimultaneousDefeat,
}

/// Why one seat won a finished game.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WinReason {
    /// The losing Avatar was defeated.
    AvatarDefeated,
    /// The losing seat attempted to draw from an empty deck.
    DeckEmpty,
}

/// Exact public terminal object for a finished game.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FinishedTerminal {
    /// Neither seat won.
    Draw {
        /// Why the draw occurred.
        reason: DrawReason,
        /// Public draw marker.
        result: DrawResult,
        /// Finished status marker.
        status: FinishedStatus,
    },
    /// Exactly one seat won.
    Win {
        /// Losing seat.
        loser: Seat,
        /// Why the seat lost.
        reason: WinReason,
        /// Finished status marker.
        status: FinishedStatus,
        /// Winning seat.
        winner: Seat,
    },
}

impl FinishedTerminal {
    pub(crate) fn from_game(outcome: GameOutcome, reason: GameEndReason) -> Option<Self> {
        match (outcome, reason) {
            (GameOutcome::Draw, GameEndReason::SimultaneousAvatarDefeat) => Some(Self::Draw {
                reason: DrawReason::SimultaneousAvatarDefeat,
                result: DrawResult::Draw,
                status: FinishedStatus::Finished,
            }),
            (GameOutcome::Draw, GameEndReason::SimultaneousDefeat) => Some(Self::Draw {
                reason: DrawReason::SimultaneousDefeat,
                result: DrawResult::Draw,
                status: FinishedStatus::Finished,
            }),
            (GameOutcome::Win { loser, winner }, GameEndReason::AvatarDefeated) => {
                Some(Self::Win {
                    loser,
                    reason: WinReason::AvatarDefeated,
                    status: FinishedStatus::Finished,
                    winner,
                })
            }
            (GameOutcome::Win { loser, winner }, GameEndReason::DeckEmpty) => Some(Self::Win {
                loser,
                reason: WinReason::DeckEmpty,
                status: FinishedStatus::Finished,
                winner,
            }),
            (GameOutcome::Draw, GameEndReason::AvatarDefeated | GameEndReason::DeckEmpty)
            | (
                GameOutcome::Win { .. },
                GameEndReason::SimultaneousAvatarDefeat | GameEndReason::SimultaneousDefeat,
            ) => None,
        }
    }

    /// Returns the winner when the terminal is a win.
    #[must_use]
    pub const fn winner(self) -> Option<Seat> {
        match self {
            Self::Draw { .. } => None,
            Self::Win { winner, .. } => Some(winner),
        }
    }
}

/// Compact authoritative result for one ordered batch job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResult {
    /// Original zero-based job index.
    pub job_index: usize,
    /// Canonical manifest identity.
    pub manifest_id: IdentityHash,
    /// Number of accepted actions.
    pub accepted_action_count: usize,
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Final authoritative state identity.
    pub final_state_hash: IdentityHash,
    /// Number of fights started.
    pub fight_count: usize,
    /// Exact finished public terminal.
    pub terminal: FinishedTerminal,
    /// Whether final authoritative replay reproduced the rollout.
    pub replay_verified: bool,
    /// Final authoritative transcript identity.
    pub transcript_hash: IdentityHash,
    /// Final turn number.
    pub turn_count: u64,
}

/// Exact externally serialized deterministic-game report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeterministicGameReport {
    /// Number of accepted actions.
    pub accepted_action_count: usize,
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Final authoritative state identity.
    pub final_state_hash: IdentityHash,
    /// Number of fights started.
    pub fight_count: usize,
    /// Whether final authoritative replay reproduced the rollout.
    pub replay_verified: bool,
    /// Exact finished public terminal.
    pub terminal: FinishedTerminal,
    /// Final authoritative transcript identity.
    pub transcript_hash: IdentityHash,
    /// Final turn number.
    pub turn_count: u64,
}

/// Exact externally serialized result for one ordered batch job.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameBatchResult {
    /// Original zero-based job index.
    pub job_index: usize,
    /// Canonical manifest identity.
    pub manifest_id: IdentityHash,
    /// Deterministic authoritative game report.
    pub report: DeterministicGameReport,
}

impl From<BatchResult> for GameBatchResult {
    fn from(result: BatchResult) -> Self {
        Self {
            job_index: result.job_index,
            manifest_id: result.manifest_id,
            report: DeterministicGameReport {
                accepted_action_count: result.accepted_action_count,
                classification: result.classification,
                final_state_hash: result.final_state_hash,
                fight_count: result.fight_count,
                replay_verified: result.replay_verified,
                terminal: result.terminal,
                transcript_hash: result.transcript_hash,
                turn_count: result.turn_count,
            },
        }
    }
}

/// A bounded batch was invalid or one job failed.
#[derive(Debug)]
pub enum BatchError {
    /// A batch-wide bound was invalid.
    Invalid(&'static str),
    /// One indexed job failed engine execution or replay.
    Job {
        /// Original job index.
        job_index: usize,
        /// Underlying engine failure.
        source: SimulatorError,
    },
    /// One indexed rollout did not finish within its action bound.
    NonTerminal(usize),
    /// A native worker panicked.
    WorkerPanicked,
    /// Writing one job's SIM-03 artifacts failed.
    Artifacts {
        /// Original job index.
        job_index: usize,
        /// Underlying record or filesystem failure.
        source: crate::game_record::GameRecordError,
    },
}

impl fmt::Display for BatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => formatter.write_str(message),
            Self::Job { job_index, source } => {
                write!(formatter, "batch job {job_index} failed: {source}")
            }
            Self::NonTerminal(job_index) => {
                write!(formatter, "batch job {job_index} did not terminate")
            }
            Self::WorkerPanicked => formatter.write_str("native batch worker panicked"),
            Self::Artifacts { job_index, source } => {
                write!(
                    formatter,
                    "batch job {job_index} artifacts failed: {source}"
                )
            }
        }
    }
}

impl Error for BatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Job { source, .. } => Some(source),
            Self::Artifacts { source, .. } => Some(source),
            Self::Invalid(_) | Self::NonTerminal(_) | Self::WorkerPanicked => None,
        }
    }
}

/// Runs a bounded batch entirely inside Rust and preserves input ordering across worker counts.
///
/// Rollouts use compact state and action indices. Every completed job is then replayed through
/// [`crate::session::Session`] to produce authoritative hashes.
///
/// # Errors
///
/// Returns [`BatchError`] when a bound, policy binding, manifest, rollout, replay, or worker fails.
pub fn run_batch(
    jobs: &[BatchJob<'_>],
    requested_workers: usize,
) -> Result<Vec<BatchResult>, BatchError> {
    run_batch_inner(jobs, requested_workers, None)
}

/// Runs a bounded batch and writes one SIM-03 artifact directory per job.
///
/// Compact reports stay in input order. Each job writes into `{dir}/{jobIndex}/`.
///
/// # Errors
///
/// Returns [`BatchError`] under the same conditions as [`run_batch`], or when
/// artifact paths or writes fail.
pub fn run_batch_to_dir(
    jobs: &[BatchJob<'_>],
    requested_workers: usize,
    artifacts_dir: &Path,
) -> Result<Vec<BatchResult>, BatchError> {
    validate_artifacts_dir(artifacts_dir).map_err(|source| BatchError::Artifacts {
        job_index: 0,
        source,
    })?;
    std::fs::create_dir_all(artifacts_dir).map_err(|error| BatchError::Artifacts {
        job_index: 0,
        source: error.into(),
    })?;
    run_batch_inner(jobs, requested_workers, Some(artifacts_dir))
}

fn run_batch_inner(
    jobs: &[BatchJob<'_>],
    requested_workers: usize,
    artifacts_dir: Option<&Path>,
) -> Result<Vec<BatchResult>, BatchError> {
    if jobs.is_empty() || jobs.len() > MAX_BATCH_JOBS {
        return Err(BatchError::Invalid("batch must contain 1-256 jobs"));
    }
    if !(1..=MAX_BATCH_WORKERS).contains(&requested_workers) {
        return Err(BatchError::Invalid("batch workers must be 1-8"));
    }
    let bytes = jobs.iter().try_fold(jobs.len() + 1, |total, job| {
        total.checked_add(job.manifest_json.len())
    });
    if bytes.is_none_or(|bytes| bytes > MAX_BATCH_BYTES) {
        return Err(BatchError::Invalid("batch exceeds 64 MiB"));
    }

    let worker_count = requested_workers.min(jobs.len());
    let chunk_size = jobs.len().div_ceil(worker_count);
    let results = thread::scope(|scope| {
        let handles = jobs
            .chunks(chunk_size)
            .enumerate()
            .map(|(chunk_index, chunk)| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .enumerate()
                        .map(|(offset, job)| {
                            finish_job(chunk_index * chunk_size + offset, job, artifacts_dir)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        let mut results = Vec::with_capacity(jobs.len());
        for handle in handles {
            results.extend(handle.join().map_err(|_| BatchError::WorkerPanicked)?);
        }
        Ok::<_, BatchError>(results)
    })?;
    results.into_iter().collect()
}

/// Runs an ordered native batch and returns the exact external report contract.
///
/// # Errors
///
/// Returns [`BatchError`] under the same conditions as [`run_batch`].
pub fn run_game_batch(
    jobs: &[BatchJob<'_>],
    requested_workers: usize,
) -> Result<Vec<GameBatchResult>, BatchError> {
    Ok(run_batch(jobs, requested_workers)?
        .into_iter()
        .map(GameBatchResult::from)
        .collect())
}

/// Runs an ordered native batch and writes one SIM-03 artifact directory per job.
///
/// # Errors
///
/// Returns [`BatchError`] under the same conditions as [`run_batch_to_dir`].
pub fn run_game_batch_to_dir(
    jobs: &[BatchJob<'_>],
    requested_workers: usize,
    artifacts_dir: &Path,
) -> Result<Vec<GameBatchResult>, BatchError> {
    Ok(run_batch_to_dir(jobs, requested_workers, artifacts_dir)?
        .into_iter()
        .map(GameBatchResult::from)
        .collect())
}

/// Returns the default bounded native worker count for this host.
#[must_use]
pub fn default_batch_workers() -> usize {
    thread::available_parallelism().map_or(1, |workers| workers.get().min(MAX_BATCH_WORKERS))
}

fn finish_job(
    job_index: usize,
    job: &BatchJob<'_>,
    artifacts_dir: Option<&Path>,
) -> Result<BatchResult, BatchError> {
    let (result, session) = run_job(job_index, job)?;
    if let Some(dir) = artifacts_dir {
        let record = game_record_from_session(&session)
            .map_err(|source| BatchError::Artifacts { job_index, source })?;
        write_game_artifacts(&dir.join(job_index.to_string()), &record)
            .map_err(|source| BatchError::Artifacts { job_index, source })?;
    }
    Ok(result)
}

fn run_job(job_index: usize, job: &BatchJob<'_>) -> Result<(BatchResult, Session), BatchError> {
    let failed = |source| BatchError::Job { job_index, source };
    let game = Game::from_manifest_json(job.manifest_json)
        .map_err(SimulatorError::from)
        .map_err(failed)?;
    for (policy, deck_id) in [
        (job.north_policy, job.north_deck_id),
        (job.south_policy, job.south_deck_id),
    ] {
        policy
            .validate_binding(
                game.rules().authority_hash(),
                deck_id,
                game.rules().engine_version(),
            )
            .map_err(SimulatorError::from)
            .map_err(failed)?;
    }
    let rollout =
        run_game(game, job.north_policy, job.south_policy, MAX_GAME_ACTIONS).map_err(failed)?;
    if rollout.outcome().is_none() {
        return Err(BatchError::NonTerminal(job_index));
    }
    let session = replay_selected(job.manifest_json, &rollout).map_err(failed)?;
    let (Some(outcome), Some(reason)) = (session.outcome(), session.terminal_reason()) else {
        return Err(BatchError::NonTerminal(job_index));
    };
    let Some(terminal) = FinishedTerminal::from_game(outcome, reason) else {
        return Err(BatchError::Invalid(
            "game terminal outcome and reason disagree",
        ));
    };
    let fight_count = session
        .transcript()
        .iter()
        .flat_map(|receipt| &receipt.events)
        .filter(|event| event.event_type == "fight-started")
        .count();
    Ok((
        BatchResult {
            job_index,
            manifest_id: session.manifest_id().clone(),
            accepted_action_count: session.transcript().len(),
            classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
            final_state_hash: session
                .state_hash()
                .map_err(SessionError::from)
                .map_err(SimulatorError::from)
                .map_err(failed)?,
            fight_count,
            terminal,
            replay_verified: true,
            transcript_hash: session
                .transcript_hash()
                .map_err(SimulatorError::from)
                .map_err(failed)?,
            turn_count: session.turn_number(),
        },
        session,
    ))
}
