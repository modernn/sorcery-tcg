//! One-step novelty search over engine-issued legal actions.

use serde_json::Value;

use crate::canonical::IdentityHash;
use crate::game::{Game, IssuedAction};
use crate::policy::PolicySnapshot;
use crate::simulator::SimulatorError;

/// Widest legal-action list that still probes every branch.
pub const NOVELTY_WIDTH_LIMIT: usize = 128;

/// One speculative application of an engine-issued action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoveltyProbe {
    action_id: IdentityHash,
    action_index: usize,
    action_kind: String,
    event_types: Vec<String>,
    new_action_kind: bool,
    new_event_count: usize,
    post_state_hash: IdentityHash,
    selected_by_fallback: bool,
}

/// The selected probe and every branch scored at this decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoveltyStep {
    probes: Vec<NoveltyProbe>,
    selected_index: usize,
    too_wide: bool,
}

impl NoveltyProbe {
    /// Returns the engine-issued action identity.
    #[must_use]
    pub const fn action_id(&self) -> &IdentityHash {
        &self.action_id
    }

    /// Returns the canonical legal-action index.
    #[must_use]
    pub const fn action_index(&self) -> usize {
        self.action_index
    }

    /// Returns the descriptor kind.
    #[must_use]
    pub fn action_kind(&self) -> &str {
        &self.action_kind
    }

    /// Returns sorted unique event types produced by the probe.
    #[must_use]
    pub fn event_types(&self) -> &[String] {
        &self.event_types
    }

    /// Returns whether this kind is absent from committed coverage.
    #[must_use]
    pub const fn new_action_kind(&self) -> bool {
        self.new_action_kind
    }

    /// Returns how many probed event types are absent from committed coverage.
    #[must_use]
    pub const fn new_event_count(&self) -> usize {
        self.new_event_count
    }

    /// Returns the authoritative state hash after the probe.
    #[must_use]
    pub const fn post_state_hash(&self) -> &IdentityHash {
        &self.post_state_hash
    }

    /// Returns whether the baseline policy also selected this action.
    #[must_use]
    pub const fn selected_by_fallback(&self) -> bool {
        self.selected_by_fallback
    }
}

impl NoveltyStep {
    /// Returns every scored probe in canonical action order.
    #[must_use]
    pub fn probes(&self) -> &[NoveltyProbe] {
        &self.probes
    }

    /// Returns the index into [`Self::probes`] of the selected branch.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Returns whether only the baseline-policy fallback was probed.
    #[must_use]
    pub const fn too_wide(&self) -> bool {
        self.too_wide
    }

    /// Returns the selected probe.
    #[must_use]
    pub fn selected(&self) -> Option<&NoveltyProbe> {
        self.probes.get(self.selected_index)
    }
}

/// Probes engine-issued actions and selects the one-step novelty-v1 winner.
///
/// Wider than [`NOVELTY_WIDTH_LIMIT`] lists probe only the baseline-policy fallback.
/// Selection prefers more new event types, then a new action kind, then the fallback,
/// then canonical order.
///
/// # Errors
///
/// Returns [`SimulatorError`] when the position has no legal actions or a probe fails.
pub fn probe_novelty(
    game: &Game,
    policy: &PolicySnapshot,
    committed_action_kinds: &[String],
    committed_event_types: &[String],
) -> Result<NoveltyStep, SimulatorError> {
    let actions = game.legal_actions()?;
    if actions.is_empty() {
        return Err(SimulatorError::InvalidLimit);
    }
    let seat = game.position().decision_seat();
    let fallback = policy.select_action(&game.observe(seat), &actions)?;
    let fallback_index = actions
        .iter()
        .position(|action| action == fallback)
        .ok_or(SimulatorError::ReplayDiverged)?;
    let too_wide = actions.len() > NOVELTY_WIDTH_LIMIT;
    let mut probes = Vec::new();
    if too_wide {
        probes.push(probe_one(
            game,
            &actions,
            fallback_index,
            fallback_index,
            committed_action_kinds,
            committed_event_types,
        )?);
    } else {
        for index in 0..actions.len() {
            probes.push(probe_one(
                game,
                &actions,
                index,
                fallback_index,
                committed_action_kinds,
                committed_event_types,
            )?);
        }
    }
    let selected_index = probes
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| compare_probes(left, right))
        .map(|(index, _)| index)
        .ok_or(SimulatorError::InvalidLimit)?;
    Ok(NoveltyStep {
        probes,
        selected_index,
        too_wide,
    })
}

fn probe_one(
    game: &Game,
    actions: &[IssuedAction],
    index: usize,
    fallback_index: usize,
    committed_action_kinds: &[String],
    committed_event_types: &[String],
) -> Result<NoveltyProbe, SimulatorError> {
    let action = &actions[index];
    let legal = action.to_legal_action()?;
    let kind = action_kind(&legal.descriptor)?;
    let mut branch = game.clone();
    let (outcomes, _) = branch.apply_action_recorded(action)?;
    let mut event_types = outcomes
        .into_iter()
        .map(|(event_type, _)| event_type)
        .collect::<Vec<_>>();
    event_types.sort();
    event_types.dedup();
    let new_event_count = event_types
        .iter()
        .filter(|event_type| {
            !committed_event_types
                .iter()
                .any(|committed| committed == *event_type)
        })
        .count();
    Ok(NoveltyProbe {
        action_id: legal.action_id,
        action_index: index,
        new_action_kind: !committed_action_kinds
            .iter()
            .any(|committed| committed == &kind),
        new_event_count,
        action_kind: kind,
        event_types,
        post_state_hash: branch.state_hash().map_err(crate::game::GameError::from)?,
        selected_by_fallback: index == fallback_index,
    })
}

fn compare_probes(left: &NoveltyProbe, right: &NoveltyProbe) -> std::cmp::Ordering {
    right
        .new_event_count
        .cmp(&left.new_event_count)
        .then_with(|| right.new_action_kind.cmp(&left.new_action_kind))
        .then_with(|| right.selected_by_fallback.cmp(&left.selected_by_fallback))
        .then_with(|| left.action_index.cmp(&right.action_index))
}

fn action_kind(descriptor: &Value) -> Result<String, SimulatorError> {
    match descriptor.get("kind") {
        Some(Value::String(kind)) => Ok(kind.clone()),
        _ => Err(SimulatorError::ReplayDiverged),
    }
}

#[cfg(test)]
mod tests {
    use super::{NoveltyProbe, compare_probes};
    use crate::canonical::IdentityHash;

    fn probe(
        index: usize,
        new_event_count: usize,
        new_action_kind: bool,
        selected_by_fallback: bool,
    ) -> NoveltyProbe {
        NoveltyProbe {
            action_id: IdentityHash::parse(&format!("sha256:{}", "a".repeat(64))).expect("id"),
            action_index: index,
            action_kind: "draw".to_owned(),
            event_types: Vec::new(),
            new_action_kind,
            new_event_count,
            post_state_hash: IdentityHash::parse(&format!("sha256:{}", "b".repeat(64)))
                .expect("hash"),
            selected_by_fallback,
        }
    }

    #[test]
    fn novelty_prefers_new_events_then_kind_then_fallback_then_order() {
        assert_eq!(
            compare_probes(&probe(1, 2, false, false), &probe(0, 1, true, true)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_probes(&probe(1, 1, true, false), &probe(0, 1, false, true)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_probes(&probe(1, 1, true, true), &probe(0, 1, true, false)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_probes(&probe(0, 1, true, true), &probe(1, 1, true, true)),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn probe_novelty_scores_synthetic_opening() {
        use crate::game::Game;
        use crate::policy::baseline_policy_snapshot;
        use crate::synthetic::synthetic_demo_manifest_json;

        let game = Game::from_manifest_json(
            &synthetic_demo_manifest_json(31).expect("synthetic manifest"),
        )
        .expect("game");
        let policy =
            baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
                .expect("policy");
        let step = super::probe_novelty(&game, &policy, &[], &[]).expect("novelty");
        let actions = game.legal_actions().expect("legal actions");
        assert!(!step.too_wide());
        assert_eq!(step.probes().len(), actions.len());
        assert_eq!(
            step.probes()
                .iter()
                .filter(|candidate| candidate.selected_by_fallback())
                .count(),
            1
        );
        let winner = step
            .probes()
            .iter()
            .min_by(|left, right| compare_probes(left, right))
            .expect("winner");
        assert_eq!(step.selected(), Some(winner));
    }
}
