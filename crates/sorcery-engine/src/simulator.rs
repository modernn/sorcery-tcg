//! Deterministic policy rollouts and bounded root-action search.

use std::error::Error;
use std::fmt;

use crate::canonical::IdentityHash;
use crate::contract::{ActionRequest, Seat};
use crate::game::{Game, GameError, GameOutcome};
use crate::policy::{PolicyError, PolicySnapshot};
use crate::session::{Session, SessionError};

/// One compact rollout result, without receipts or replay journals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rollout {
    action_indices: Vec<usize>,
    manifest_id: IdentityHash,
    outcome: Option<GameOutcome>,
    terminal: bool,
}

/// Branches rooted at one exact authoritative session checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointSearch {
    root_session_hash: IdentityHash,
    rollouts: Vec<Rollout>,
}

impl CheckpointSearch {
    /// Returns canonically ordered checkpoint branches.
    #[must_use]
    pub fn rollouts(&self) -> &[Rollout] {
        &self.rollouts
    }
}

impl Rollout {
    /// Returns canonical legal-action indices in application order.
    #[must_use]
    pub fn action_indices(&self) -> &[usize] {
        &self.action_indices
    }

    /// Returns whether the rollout reached an authoritative terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }

    /// Returns the public terminal result, when the rollout finished.
    #[must_use]
    pub const fn outcome(&self) -> Option<GameOutcome> {
        self.outcome
    }
}

/// Rollout, policy selection, or authoritative replay failed.
#[derive(Debug)]
pub enum SimulatorError {
    /// An engine operation failed.
    Game(GameError),
    /// A policy could not select from the engine-issued actions.
    Policy(PolicyError),
    /// Authoritative replay failed.
    Session(SessionError),
    /// A search bound was zero.
    InvalidLimit,
    /// Authoritative replay did not reproduce the rollout's final state.
    ReplayDiverged,
}

impl fmt::Display for SimulatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(error) => error.fmt(formatter),
            Self::Policy(error) => error.fmt(formatter),
            Self::Session(error) => error.fmt(formatter),
            Self::InvalidLimit => {
                formatter.write_str("simulation limits must be greater than zero")
            }
            Self::ReplayDiverged => {
                formatter.write_str("authoritative replay diverged from the selected rollout")
            }
        }
    }
}

impl Error for SimulatorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Game(error) => Some(error),
            Self::Policy(error) => Some(error),
            Self::Session(error) => Some(error),
            Self::InvalidLimit | Self::ReplayDiverged => None,
        }
    }
}

impl From<GameError> for SimulatorError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

impl From<PolicyError> for SimulatorError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

impl From<SessionError> for SimulatorError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

/// Runs one policy-controlled game without building receipts or journals.
///
/// The result is nonterminal when `max_actions` is exhausted.
///
/// # Errors
///
/// Returns [`SimulatorError`] when the bound is zero or the engine or policy fails.
pub fn run_game(
    game: Game,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    max_actions: usize,
) -> Result<Rollout, SimulatorError> {
    if max_actions == 0 {
        return Err(SimulatorError::InvalidLimit);
    }
    continue_game(game, north_policy, south_policy, Vec::new(), max_actions)
}

/// Branches from the first `max_root_actions` legal actions in canonical order.
///
/// Every branch uses a compact [`Game`] clone and follows the policies after its
/// forced root action. No receipts, journals, serialized checkpoints, or per-node state hashes are
/// built.
///
/// # Errors
///
/// Returns [`SimulatorError`] when either bound is zero or the engine or policy fails.
pub fn search_root_actions(
    game: &Game,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    max_actions: usize,
    max_root_actions: usize,
) -> Result<Vec<Rollout>, SimulatorError> {
    if max_actions == 0 || max_root_actions == 0 {
        return Err(SimulatorError::InvalidLimit);
    }
    let root_actions = game.legal_actions()?;
    let mut rollouts = Vec::with_capacity(root_actions.len().min(max_root_actions));
    for (action_index, action) in root_actions.into_iter().take(max_root_actions).enumerate() {
        let mut branch = game.clone();
        branch.apply_action(&action)?;
        rollouts.push(continue_game(
            branch,
            north_policy,
            south_policy,
            vec![action_index],
            max_actions - 1,
        )?);
    }
    Ok(rollouts)
}

/// Branches directly from an authoritative session without serializing its checkpoint.
///
/// The root session is hashed once. Speculative nodes remain compact clones with no receipts,
/// journals, serialization, or hashing.
///
/// # Errors
///
/// Returns [`SimulatorError`] when checkpoint hashing, search, policy selection, or a limit fails.
pub fn search_from_checkpoint(
    session: &Session,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    max_actions: usize,
    max_root_actions: usize,
) -> Result<CheckpointSearch, SimulatorError> {
    Ok(CheckpointSearch {
        root_session_hash: session.session_hash()?,
        rollouts: search_root_actions(
            &session.game_clone(),
            north_policy,
            south_policy,
            max_actions,
            max_root_actions,
        )?,
    })
}

/// Replays one selected rollout through the authoritative receipt path.
///
/// # Errors
///
/// Returns [`SimulatorError`] when replay rejects an action or does not reproduce
/// the speculative rollout's final state exactly.
pub fn replay_selected(manifest_json: &str, rollout: &Rollout) -> Result<Session, SimulatorError> {
    apply_and_verify(Session::new(manifest_json)?, rollout)
}

/// Replays one selected branch from its exact authoritative checkpoint.
///
/// # Errors
///
/// Returns [`SimulatorError`] when the checkpoint differs, the branch index is invalid,
/// or authoritative replay diverges.
pub fn replay_checkpoint_branch(
    session: &Session,
    search: &CheckpointSearch,
    branch_index: usize,
) -> Result<Session, SimulatorError> {
    if session.session_hash()? != search.root_session_hash {
        return Err(SimulatorError::ReplayDiverged);
    }
    let rollout = search
        .rollouts
        .get(branch_index)
        .ok_or(SimulatorError::ReplayDiverged)?;
    apply_and_verify(session.clone(), rollout)
}

fn apply_and_verify(mut session: Session, rollout: &Rollout) -> Result<Session, SimulatorError> {
    if session.manifest_id() != &rollout.manifest_id {
        return Err(SimulatorError::ReplayDiverged);
    }
    for &action_index in rollout.action_indices() {
        let action = session
            .legal_actions()?
            .into_iter()
            .nth(action_index)
            .ok_or(SimulatorError::ReplayDiverged)?;
        if matches!(
            session.step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: session.decision_seat(),
                state_version: session.state_version(),
            })?,
            crate::session::StepResult::Rejected(_)
        ) {
            return Err(SimulatorError::ReplayDiverged);
        }
    }
    if !session.verify_replay()? {
        return Err(SimulatorError::ReplayDiverged);
    }
    if session.outcome() != rollout.outcome {
        return Err(SimulatorError::ReplayDiverged);
    }
    Ok(session)
}

fn continue_game(
    mut game: Game,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    mut action_indices: Vec<usize>,
    remaining_actions: usize,
) -> Result<Rollout, SimulatorError> {
    for _ in 0..remaining_actions {
        if game.is_terminal() {
            break;
        }
        let seat = game.position().decision_seat();
        let actions = game.legal_actions()?;
        let selected = policy_for(seat, north_policy, south_policy)
            .select_action(game.observe(seat), &actions)?;
        let selected_index = actions
            .iter()
            .position(|action| std::ptr::eq(action, selected))
            .ok_or(SimulatorError::ReplayDiverged)?;
        action_indices.push(selected_index);
        game.apply_action(selected)?;
    }
    Ok(Rollout {
        action_indices,
        manifest_id: game.rules().manifest_id().clone(),
        outcome: game.outcome(),
        terminal: game.is_terminal(),
    })
}

const fn policy_for<'a>(
    seat: Seat,
    north: &'a PolicySnapshot,
    south: &'a PolicySnapshot,
) -> &'a PolicySnapshot {
    match seat {
        Seat::North => north,
        Seat::South => south,
    }
}
