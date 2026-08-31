//! Ordered native rollout batches with authoritative final replay.

use std::error::Error;
use std::fmt;
use std::thread;

use crate::canonical::IdentityHash;
use crate::game::{Game, GameOutcome};
use crate::policy::PolicySnapshot;
use crate::session::SessionError;
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchClassification {
    /// The game was exact for exercised mechanics but is not ranked-eligible.
    UnrankedPartialRules,
}

/// One complete native rollout job.
#[derive(Clone, Copy, Debug)]
pub struct BatchJob<'a> {
    /// Canonical authoritative manifest bytes.
    pub manifest_json: &'a str,
    /// North's immutable deck-bound policy.
    pub north_policy: &'a PolicySnapshot,
    /// South's immutable deck-bound policy.
    pub south_policy: &'a PolicySnapshot,
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
    /// Terminal public result.
    pub outcome: GameOutcome,
    /// Whether final authoritative replay reproduced the rollout.
    pub replay_verified: bool,
    /// Final authoritative transcript identity.
    pub transcript_hash: IdentityHash,
    /// Final turn number.
    pub turn_count: u64,
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
        }
    }
}

impl Error for BatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Job { source, .. } => Some(source),
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
    max_actions: usize,
    requested_workers: usize,
) -> Result<Vec<BatchResult>, BatchError> {
    if jobs.is_empty() || jobs.len() > MAX_BATCH_JOBS {
        return Err(BatchError::Invalid("batch must contain 1-256 jobs"));
    }
    if max_actions == 0 {
        return Err(BatchError::Invalid(
            "batch action bound must be greater than zero",
        ));
    }
    if !(1..=MAX_BATCH_WORKERS).contains(&requested_workers) {
        return Err(BatchError::Invalid("batch workers must be 1-8"));
    }
    let bytes = jobs.iter().try_fold(0_usize, |total, job| {
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
                            run_job(chunk_index * chunk_size + offset, job, max_actions)
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

fn run_job(
    job_index: usize,
    job: &BatchJob<'_>,
    max_actions: usize,
) -> Result<BatchResult, BatchError> {
    let failed = |source| BatchError::Job { job_index, source };
    let game = Game::from_manifest_json(job.manifest_json)
        .map_err(SimulatorError::from)
        .map_err(failed)?;
    for policy in [job.north_policy, job.south_policy] {
        policy
            .validate_binding(
                game.rules().authority_hash(),
                policy.deck_id(),
                game.rules().engine_version(),
            )
            .map_err(SimulatorError::from)
            .map_err(failed)?;
    }
    let rollout =
        run_game(game, job.north_policy, job.south_policy, max_actions).map_err(failed)?;
    let Some(outcome) = rollout.outcome() else {
        return Err(BatchError::NonTerminal(job_index));
    };
    let session = replay_selected(job.manifest_json, &rollout).map_err(failed)?;
    let fight_count = session
        .transcript()
        .iter()
        .flat_map(|receipt| &receipt.events)
        .filter(|event| event.event_type == "fight-started")
        .count();
    Ok(BatchResult {
        job_index,
        manifest_id: session.manifest_id().clone(),
        accepted_action_count: session.transcript().len(),
        classification: BatchClassification::UnrankedPartialRules,
        final_state_hash: session
            .state_hash()
            .map_err(SessionError::from)
            .map_err(SimulatorError::from)
            .map_err(failed)?,
        fight_count,
        outcome,
        replay_verified: true,
        transcript_hash: session
            .transcript_hash()
            .map_err(SimulatorError::from)
            .map_err(failed)?,
        turn_count: session.turn_number(),
    })
}
