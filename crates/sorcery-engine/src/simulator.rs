//! Deterministic policy rollouts and bounded root-action search.

use std::error::Error;
use std::fmt;

use crate::canonical::IdentityHash;
use crate::contract::Seat;
use crate::game::{Game, GameError};
use crate::policy::{PolicyError, PolicySnapshot};
use crate::session::{Session, SessionError};

/// One compact rollout result, without receipts or replay journals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rollout {
    action_ids: Vec<IdentityHash>,
    final_state_hash: IdentityHash,
    terminal: bool,
}

impl Rollout {
    /// Returns engine-issued action identities in application order.
    #[must_use]
    pub fn action_ids(&self) -> &[IdentityHash] {
        &self.action_ids
    }

    /// Returns the final state identity, materialized once after the rollout.
    #[must_use]
    pub const fn final_state_hash(&self) -> &IdentityHash {
        &self.final_state_hash
    }

    /// Returns whether the rollout reached an authoritative terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
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
    for action in root_actions.into_iter().take(max_root_actions) {
        let mut branch = game.clone();
        let action_id = action.action_id().clone();
        branch.apply_action(&action)?;
        rollouts.push(continue_game(
            branch,
            north_policy,
            south_policy,
            vec![action_id],
            max_actions - 1,
        )?);
    }
    Ok(rollouts)
}

/// Replays one selected rollout through the authoritative receipt path.
///
/// # Errors
///
/// Returns [`SimulatorError`] when replay rejects an action or does not reproduce
/// the speculative rollout's final state exactly.
pub fn replay_selected(manifest_json: &str, rollout: &Rollout) -> Result<Session, SimulatorError> {
    let session = Session::replay(manifest_json, rollout.action_ids())?;
    if session.state_hash().map_err(SessionError::Canonical)? != *rollout.final_state_hash()
        || !session.verify_replay()?
    {
        return Err(SimulatorError::ReplayDiverged);
    }
    Ok(session)
}

fn continue_game(
    mut game: Game,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    mut action_ids: Vec<IdentityHash>,
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
        action_ids.push(selected.action_id().clone());
        game.apply_action(selected)?;
    }
    Ok(Rollout {
        action_ids,
        final_state_hash: game.state_hash().map_err(GameError::Canonical)?,
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
