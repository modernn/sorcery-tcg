//! Deterministic policy rollouts and bounded root-action search.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::canonical::IdentityHash;
use crate::contract::{ActionRequest, Seat};
use crate::game::{Game, GameError, GameOutcome, IssuedAction, Position};
use crate::policy::{PolicyError, PolicySnapshot};
use crate::session::{Session, SessionError};

/// One compact rollout result, without receipts or replay journals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rollout {
    action_indices: Vec<usize>,
    final_position: Position,
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

/// An advisory exploration prefix bound to one exact local session.
///
/// These fields identify existing actions; they cannot create actions or mutate state.
/// Keep this binding local when sending a separate seat-scoped observation to a model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RootActionOrder {
    /// Authoritative session identity, including its journals.
    pub root_session_hash: IdentityHash,
    /// Version that issued the candidate actions.
    pub state_version: u64,
    /// Unique engine-issued action IDs in preferred exploration order.
    /// Unlisted actions follow in their original canonical order.
    pub action_ids: Vec<IdentityHash>,
}

impl CheckpointSearch {
    /// Returns checkpoint branches in exploration order.
    /// Each rollout still records original canonical legal-action indices.
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
    /// An exploration prefix was stale or did not name unique issued actions.
    InvalidRootActionOrder(&'static str),
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
            Self::InvalidRootActionOrder(reason) => formatter.write_str(reason),
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
            Self::InvalidLimit | Self::InvalidRootActionOrder(_) | Self::ReplayDiverged => None,
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
    let order = (0..root_actions.len()).collect::<Vec<_>>();
    search_issued_root_actions(
        game,
        north_policy,
        south_policy,
        max_actions,
        max_root_actions,
        &root_actions,
        &order,
    )
}

fn search_issued_root_actions(
    game: &Game,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    max_actions: usize,
    max_root_actions: usize,
    root_actions: &[IssuedAction],
    order: &[usize],
) -> Result<Vec<Rollout>, SimulatorError> {
    let mut rollouts = Vec::with_capacity(root_actions.len().min(max_root_actions));
    for &action_index in order.iter().take(max_root_actions) {
        let branch = game
            .clone()
            .apply_action_owned(&root_actions[action_index])?;
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
    search_from_checkpoint_with_order(
        session,
        north_policy,
        south_policy,
        max_actions,
        max_root_actions,
        None,
    )
}

/// Explores an optional engine-issued action prefix from an exact checkpoint.
///
/// The complete prefix is validated before any branch runs, including entries beyond
/// `max_root_actions`. Unlisted actions follow canonically. `None` preserves the
/// ordinary search order. No model is required, and the caller's session is unchanged.
/// This changes exploration order only; it does not score unfinished branches or
/// establish that a selected action is optimal.
///
/// # Errors
///
/// Returns [`SimulatorError`] for zero bounds, a stale/invalid prefix, or an engine,
/// policy, or checkpoint-identity failure.
pub fn search_from_checkpoint_with_order(
    session: &Session,
    north_policy: &PolicySnapshot,
    south_policy: &PolicySnapshot,
    max_actions: usize,
    max_root_actions: usize,
    order: Option<&RootActionOrder>,
) -> Result<CheckpointSearch, SimulatorError> {
    if max_actions == 0 || max_root_actions == 0 {
        return Err(SimulatorError::InvalidLimit);
    }
    let root_session_hash = session.session_hash()?;
    if let Some(order) = order
        && (order.root_session_hash != root_session_hash
            || order.state_version != session.state_version())
    {
        return Err(SimulatorError::InvalidRootActionOrder(
            "root action order belongs to a different session or state version",
        ));
    }
    let game = session.game_clone();
    let root_actions = game.legal_actions()?;
    let indices = if let Some(order) = order {
        if order.action_ids.len() > root_actions.len() {
            return Err(SimulatorError::InvalidRootActionOrder(
                "root action order is longer than the issued action set",
            ));
        }
        let by_id = root_actions
            .iter()
            .enumerate()
            .map(|(index, action)| Ok((action.to_legal_action()?.action_id, index)))
            .collect::<Result<BTreeMap<_, _>, GameError>>()?;
        let mut selected = vec![false; root_actions.len()];
        let mut indices = Vec::with_capacity(root_actions.len());
        for id in &order.action_ids {
            let Some(&index) = by_id.get(id) else {
                return Err(SimulatorError::InvalidRootActionOrder(
                    "root action order contains an unknown action ID",
                ));
            };
            if selected[index] {
                return Err(SimulatorError::InvalidRootActionOrder(
                    "root action order contains a duplicate action ID",
                ));
            }
            selected[index] = true;
            indices.push(index);
        }
        indices.extend((0..root_actions.len()).filter(|&index| !selected[index]));
        indices
    } else {
        (0..root_actions.len()).collect()
    };
    Ok(CheckpointSearch {
        root_session_hash,
        rollouts: search_issued_root_actions(
            &game,
            north_policy,
            south_policy,
            max_actions,
            max_root_actions,
            &root_actions,
            &indices,
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
    replay_selected_from_session(Session::new(manifest_json)?, rollout)
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
    replay_selected_from_session(session.clone(), rollout)
}

pub(crate) fn replay_selected_from_session(
    mut session: Session,
    rollout: &Rollout,
) -> Result<Session, SimulatorError> {
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
                seat: session.acting_controller(),
                state_version: session.state_version(),
            })?,
            crate::session::StepResult::Rejected(_)
        ) {
            return Err(SimulatorError::ReplayDiverged);
        }
    }
    if session.position() != &rollout.final_position || !session.verify_replay()? {
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
        let seat = game.acting_controller();
        let actions = game.legal_actions()?;
        let selected = policy_for(seat, north_policy, south_policy)
            .select_action(&game.observe(seat), &actions)?;
        let selected_index = actions
            .iter()
            .position(|action| std::ptr::eq(action, selected))
            .ok_or(SimulatorError::ReplayDiverged)?;
        action_indices.push(selected_index);
        game = game.apply_action_owned(selected)?;
    }
    let manifest_id = game.rules().manifest_id().clone();
    let outcome = game.outcome();
    let terminal = game.is_terminal();
    Ok(Rollout {
        action_indices,
        final_position: game.into_position(),
        manifest_id,
        outcome,
        terminal,
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::canonical::{canonical_json, identity_hash};
    use crate::policy::parse_policy_snapshot;
    use crate::synthetic::synthetic_demo_manifest_json;

    use super::{Game, SimulatorError, replay_selected, run_game};

    #[test]
    fn replay_should_reject_a_same_outcome_with_a_different_final_position() {
        let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
        let raw: Value = serde_json::from_str(&manifest).expect("manifest JSON");
        let mut body = json!({
            "authorityHash": raw["authority"]["contentHash"],
            "deckId": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "engineVersion": raw["engineVersion"],
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
        body["policyId"] = json!(identity_hash(&body).expect("policy identity"));
        let policy = parse_policy_snapshot(&canonical_json(&body).expect("canonical policy"))
            .expect("valid policy");
        let mut rollout = run_game(
            Game::from_manifest_json(&manifest).expect("game"),
            &policy,
            &policy,
            500,
        )
        .expect("terminal rollout");
        assert!(rollout.is_terminal());
        rollout.final_position = Game::from_manifest_json(&manifest)
            .expect("different position")
            .into_position();

        assert!(matches!(
            replay_selected(&manifest, &rollout),
            Err(SimulatorError::ReplayDiverged)
        ));
    }
}
