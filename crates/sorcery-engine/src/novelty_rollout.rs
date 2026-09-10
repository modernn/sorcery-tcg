//! Coverage-guided one-step novelty rollouts over engine-issued actions.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{Value, json};

use crate::canonical::IdentityHash;
use crate::checkpoint::{
    GameCheckpoint, create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use crate::contract::{ActionRequest, LegalAction};
use crate::novelty::{NOVELTY_WIDTH_LIMIT, NoveltyProbe};
use crate::session::{Session, SessionError, StepResult};

/// Widest accepted novelty-rollout horizon.
pub const NOVELTY_ROLLOUT_ACTION_LIMIT: usize = 500;

/// Public novelty report plus checkpoints emitted for TypeScript sinks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoveltyRolloutOutput {
    emitted_checkpoints: Vec<GameCheckpoint>,
    result: Value,
}

impl NoveltyRolloutOutput {
    /// Returns the public report object.
    #[must_use]
    pub const fn result(&self) -> &Value {
        &self.result
    }

    /// Returns checkpoints in first-emission order.
    #[must_use]
    pub fn emitted_checkpoints(&self) -> &[GameCheckpoint] {
        &self.emitted_checkpoints
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NoveltyPosition {
    decision_index: u64,
    state_hash: IdentityHash,
    transcript_length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FirstSeen<T> {
    first_seen: NoveltyPosition,
    value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Signal {
    kind: &'static str,
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RankedFrontier {
    action_id: IdentityHash,
    action_kind: String,
    checkpoint_id: IdentityHash,
    decision_index: u64,
    legal_index: usize,
    new_action_kind: bool,
    new_event_count: usize,
    predicted_event_types: Vec<String>,
    predicted_state_hash: IdentityHash,
    selected_by_fallback: bool,
    signal: Signal,
}

struct Rollout {
    accepted_action_count: u64,
    branch_factors: Vec<FirstSeen<usize>>,
    committed_action_kinds: Vec<FirstSeen<String>>,
    committed_event_types: Vec<FirstSeen<String>>,
    emitted_checkpoints: Vec<GameCheckpoint>,
    emitted_ids: HashSet<IdentityHash>,
    frontier: HashMap<String, RankedFrontier>,
    initial_state_hash: IdentityHash,
    live: Session,
    max_actions: usize,
    offered_action_kinds: Vec<FirstSeen<String>>,
    probed_action_kinds: BTreeSet<String>,
    probed_event_types: BTreeSet<String>,
    replay: Session,
    seen_branch_factors: HashSet<usize>,
    seen_committed_events: HashSet<String>,
    seen_committed_kinds: HashSet<String>,
    seen_offered_kinds: HashSet<String>,
    too_wide: Vec<NoveltyPosition>,
}

/// Runs the one-step novelty-v1 rollout from an authoritative session snapshot.
///
/// The caller's session is not mutated. Live and replay clones commit the selected
/// branch; speculative probes stay on compact [`crate::game::Game`] clones.
///
/// # Errors
///
/// Returns [`SessionError`] when `max_actions` is outside `0..=500` or report
/// materialization fails.
pub fn run_novelty_rollout(
    root: &Session,
    max_actions: usize,
) -> Result<NoveltyRolloutOutput, SessionError> {
    if max_actions > NOVELTY_ROLLOUT_ACTION_LIMIT {
        return Err(SessionError::Novelty {
            message: format!("maxActions must be 0-{NOVELTY_ROLLOUT_ACTION_LIMIT}"),
        });
    }
    Rollout::new(root, max_actions)?.run()
}

impl Rollout {
    fn new(root: &Session, max_actions: usize) -> Result<Self, SessionError> {
        let live = root.clone();
        Ok(Self {
            accepted_action_count: 0,
            branch_factors: Vec::new(),
            committed_action_kinds: Vec::new(),
            committed_event_types: Vec::new(),
            emitted_checkpoints: Vec::new(),
            emitted_ids: HashSet::new(),
            frontier: HashMap::new(),
            initial_state_hash: live.state_hash()?,
            live,
            max_actions,
            offered_action_kinds: Vec::new(),
            probed_action_kinds: BTreeSet::new(),
            probed_event_types: BTreeSet::new(),
            replay: root.clone(),
            seen_branch_factors: HashSet::new(),
            seen_committed_events: HashSet::new(),
            seen_committed_kinds: HashSet::new(),
            seen_offered_kinds: HashSet::new(),
            too_wide: Vec::new(),
        })
    }

    fn run(mut self) -> Result<NoveltyRolloutOutput, SessionError> {
        if !self.live.verify_replay()? || !self.replay.verify_replay()? {
            return self.fail(json!({ "kind": "replay-mismatch" }), false);
        }
        loop {
            if self.live.outcome().is_some() {
                let extra = json!({
                    "status": "completed",
                    "terminal": terminal_value(&self.live)?,
                });
                return self.finish(true, extra);
            }
            if self.accepted_action_count
                >= u64::try_from(self.max_actions).map_err(|_| SessionError::SequenceExhausted)?
            {
                let Some(checkpoint_id) = self.try_capture()? else {
                    return self.checkpoint_failure(true);
                };
                return self.finish(
                    true,
                    json!({
                        "checkpointId": checkpoint_id,
                        "status": "horizon",
                    }),
                );
            }
            if let Some(output) = self.step_decision()? {
                return Ok(output);
            }
        }
    }

    fn step_decision(&mut self) -> Result<Option<NoveltyRolloutOutput>, SessionError> {
        let current_position = position(&self.live, self.accepted_action_count)?;
        let Ok(actions) = self.live.legal_actions() else {
            return Ok(Some(self.fail(
                json!({ "kind": "exception", "phase": "legal-actions" }),
                true,
            )?));
        };
        if self.seen_branch_factors.insert(actions.len()) {
            self.branch_factors.push(FirstSeen {
                first_seen: current_position.clone(),
                value: actions.len(),
            });
        }
        if actions.is_empty() {
            return Ok(Some(self.fail(json!({ "kind": "deadlock" }), true)?));
        }
        self.record_offered_kinds(&actions, &current_position)?;
        let (selected, probes) = match self.probe_selected(&actions, &current_position) {
            Ok(value) => value,
            Err(phase) => {
                return Ok(Some(
                    self.fail(json!({ "kind": "exception", "phase": phase }), true)?,
                ));
            }
        };
        if let Some(output) = self.record_frontier(&selected, &probes, &current_position)? {
            return Ok(Some(output));
        }
        if let Some(output) = self.commit_selected(&selected)? {
            return Ok(Some(output));
        }
        self.record_commitment(&selected, &current_position);
        self.accepted_action_count = self
            .accepted_action_count
            .checked_add(1)
            .ok_or(SessionError::SequenceExhausted)?;
        Ok(None)
    }

    fn record_offered_kinds(
        &mut self,
        actions: &[LegalAction],
        current_position: &NoveltyPosition,
    ) -> Result<(), SessionError> {
        for action in actions {
            let kind = action_kind(action)?;
            if self.seen_offered_kinds.insert(kind.clone()) {
                self.offered_action_kinds.push(FirstSeen {
                    first_seen: current_position.clone(),
                    value: kind,
                });
            }
        }
        Ok(())
    }

    fn probe_selected(
        &mut self,
        actions: &[LegalAction],
        current_position: &NoveltyPosition,
    ) -> Result<(NoveltyProbe, Vec<NoveltyProbe>), &'static str> {
        let committed_kinds = self
            .committed_action_kinds
            .iter()
            .map(|entry| entry.value.clone())
            .collect::<Vec<_>>();
        let committed_events = self
            .committed_event_types
            .iter()
            .map(|entry| entry.value.clone())
            .collect::<Vec<_>>();
        let phase = if actions.len() > NOVELTY_WIDTH_LIMIT {
            "selector"
        } else {
            "probe"
        };
        let step = self
            .live
            .probe_novelty(&committed_kinds, &committed_events)
            .map_err(|_| phase)?;
        let selected = step.selected().cloned().ok_or(phase)?;
        if !actions
            .iter()
            .any(|action| action.action_id == *selected.action_id())
        {
            return Err(phase);
        }
        if step.too_wide() {
            self.too_wide.push(current_position.clone());
        }
        for probe in step.probes() {
            self.probed_action_kinds
                .insert(probe.action_kind().to_owned());
            self.probed_event_types
                .extend(probe.event_types().iter().cloned());
        }
        Ok((selected, step.probes().to_vec()))
    }

    fn record_frontier(
        &mut self,
        selected: &NoveltyProbe,
        probes: &[NoveltyProbe],
        current_position: &NoveltyPosition,
    ) -> Result<Option<NoveltyRolloutOutput>, SessionError> {
        if probes.len() <= 1 {
            return Ok(None);
        }
        let mut committed_after_kinds = self.seen_committed_kinds.clone();
        committed_after_kinds.insert(selected.action_kind().to_owned());
        let mut committed_after_events = self.seen_committed_events.clone();
        committed_after_events.extend(selected.event_types().iter().cloned());
        let mut candidates = Vec::new();
        for probe in probes {
            if probe.action_id() == selected.action_id() {
                continue;
            }
            if !committed_after_kinds.contains(probe.action_kind()) {
                candidates.push((
                    probe,
                    Signal {
                        kind: "action-kind",
                        value: probe.action_kind().to_owned(),
                    },
                ));
            }
            for event_type in probe.event_types() {
                if !committed_after_events.contains(event_type) {
                    candidates.push((
                        probe,
                        Signal {
                            kind: "event-type",
                            value: event_type.clone(),
                        },
                    ));
                }
            }
        }
        if candidates.is_empty() {
            return Ok(None);
        }
        let Some(checkpoint_id) = self.try_capture()? else {
            return Ok(Some(self.checkpoint_failure(true)?));
        };
        for (probe, signal) in candidates {
            let ranked = RankedFrontier {
                action_id: probe.action_id().clone(),
                action_kind: probe.action_kind().to_owned(),
                checkpoint_id: checkpoint_id.clone(),
                decision_index: current_position.decision_index,
                legal_index: probe.action_index(),
                new_action_kind: probe.new_action_kind(),
                new_event_count: probe.new_event_count(),
                predicted_event_types: probe.event_types().to_vec(),
                predicted_state_hash: probe.post_state_hash().clone(),
                selected_by_fallback: probe.selected_by_fallback(),
                signal,
            };
            let key = signal_key(&ranked.signal);
            self.frontier
                .entry(key)
                .and_modify(|existing| {
                    if preferred_frontier(&ranked, existing) == Ordering::Less {
                        *existing = ranked.clone();
                    }
                })
                .or_insert(ranked);
        }
        Ok(None)
    }

    fn commit_selected(
        &mut self,
        selected: &NoveltyProbe,
    ) -> Result<Option<NoveltyRolloutOutput>, SessionError> {
        let actions = self.live.legal_actions()?;
        let selected_action = actions
            .iter()
            .find(|action| action.action_id == *selected.action_id())
            .ok_or_else(|| SessionError::Novelty {
                message: "novelty rollout lost the selected action".to_owned(),
            })?;
        let request = ActionRequest {
            action_id: selected_action.action_id.to_string(),
            seat: selected_action.seat,
            state_version: selected_action.state_version,
        };
        match self.live.step(request.clone())? {
            StepResult::Accepted(_) => {}
            StepResult::Rejected(_) => {
                return Ok(Some(self.fail(json!({ "kind": "replay-mismatch" }), true)?));
            }
        }
        let replay_step = self.replay.step(request)?;
        if self.live.state_hash()? != *selected.post_state_hash()
            || !matches!(replay_step, StepResult::Accepted(_))
            || self.live.replay_value()? != self.replay.replay_value()?
        {
            return Ok(Some(
                self.fail(json!({ "kind": "replay-mismatch" }), false)?,
            ));
        }
        Ok(None)
    }

    fn record_commitment(&mut self, selected: &NoveltyProbe, current_position: &NoveltyPosition) {
        if self
            .seen_committed_kinds
            .insert(selected.action_kind().to_owned())
        {
            self.committed_action_kinds.push(FirstSeen {
                first_seen: current_position.clone(),
                value: selected.action_kind().to_owned(),
            });
        }
        for event_type in selected.event_types() {
            if self.seen_committed_events.insert(event_type.clone()) {
                self.committed_event_types.push(FirstSeen {
                    first_seen: current_position.clone(),
                    value: event_type.clone(),
                });
            }
            self.frontier.remove(&signal_key(&Signal {
                kind: "event-type",
                value: event_type.clone(),
            }));
        }
        self.frontier.remove(&signal_key(&Signal {
            kind: "action-kind",
            value: selected.action_kind().to_owned(),
        }));
    }

    fn try_capture(&mut self) -> Result<Option<IdentityHash>, SessionError> {
        match capture_checkpoint(
            &self.live,
            &mut self.emitted_checkpoints,
            &mut self.emitted_ids,
        ) {
            Ok(checkpoint_id) => Ok(Some(checkpoint_id)),
            Err(SessionError::Novelty { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn fail(
        &mut self,
        mut reason: Value,
        replay_verified: bool,
    ) -> Result<NoveltyRolloutOutput, SessionError> {
        let Some(checkpoint_id) = self.try_capture()? else {
            return self.checkpoint_failure(replay_verified);
        };
        if reason.get("kind") != Some(&json!("checkpoint-failure")) {
            reason["checkpointId"] = json!(checkpoint_id);
        }
        self.finish(
            replay_verified,
            json!({
                "failure": reason,
                "status": "failed",
            }),
        )
    }

    fn checkpoint_failure(
        &mut self,
        replay_verified: bool,
    ) -> Result<NoveltyRolloutOutput, SessionError> {
        self.finish(
            replay_verified,
            json!({
                "failure": { "kind": "checkpoint-failure" },
                "status": "failed",
            }),
        )
    }

    fn finish(
        &self,
        replay_verified: bool,
        extra: Value,
    ) -> Result<NoveltyRolloutOutput, SessionError> {
        Ok(NoveltyRolloutOutput {
            result: summary_value(self, replay_verified, extra)?,
            emitted_checkpoints: self.emitted_checkpoints.clone(),
        })
    }
}

fn capture_checkpoint(
    session: &Session,
    emitted_checkpoints: &mut Vec<GameCheckpoint>,
    emitted_ids: &mut HashSet<IdentityHash>,
) -> Result<IdentityHash, SessionError> {
    let checkpoint = create_game_checkpoint(session).map_err(|error| novelty_error(&error))?;
    let serialized =
        serialize_game_checkpoint(&checkpoint).map_err(|error| novelty_error(&error))?;
    let parsed = parse_game_checkpoint(&serialized).map_err(|error| novelty_error(&error))?;
    let resumed = resume_game_checkpoint(&parsed).map_err(|error| novelty_error(&error))?;
    if session.replay_value()? != resumed.replay_value()? {
        return Err(SessionError::Novelty {
            message: "novelty checkpoint resume diverged".to_owned(),
        });
    }
    if emitted_ids.insert(parsed.checkpoint_id.clone()) {
        emitted_checkpoints.push(parsed.clone());
    }
    Ok(parsed.checkpoint_id)
}

fn summary_value(
    rollout: &Rollout,
    replay_verified: bool,
    extra: Value,
) -> Result<Value, SessionError> {
    let mut frontier_values = rollout.frontier.values().cloned().collect::<Vec<_>>();
    frontier_values.sort_by_key(|left| signal_key(&left.signal));
    let mut result = json!({
        "acceptedActionCount": rollout.accepted_action_count,
        "classification": "authority-private",
        "coverage": {
            "branchFactors": rollout
                .branch_factors
                .iter()
                .map(|entry| first_seen_value(&entry.first_seen, &json!(entry.value)))
                .collect::<Vec<_>>(),
            "committedActionKinds": rollout
                .committed_action_kinds
                .iter()
                .map(|entry| first_seen_value(&entry.first_seen, &json!(entry.value)))
                .collect::<Vec<_>>(),
            "committedEventTypes": rollout
                .committed_event_types
                .iter()
                .map(|entry| first_seen_value(&entry.first_seen, &json!(entry.value)))
                .collect::<Vec<_>>(),
            "offeredActionKinds": rollout
                .offered_action_kinds
                .iter()
                .map(|entry| first_seen_value(&entry.first_seen, &json!(entry.value)))
                .collect::<Vec<_>>(),
        },
        "finalStateHash": rollout.live.state_hash()?,
        "frontier": frontier_values
            .iter()
            .map(|candidate| json!({
                "actionId": candidate.action_id,
                "actionKind": candidate.action_kind,
                "checkpointId": candidate.checkpoint_id,
                "predictedEventTypes": candidate.predicted_event_types,
                "predictedStateHash": candidate.predicted_state_hash,
                "signal": {
                    "kind": candidate.signal.kind,
                    "value": candidate.signal.value,
                },
            }))
            .collect::<Vec<_>>(),
        "initialStateHash": rollout.initial_state_hash,
        "manifestId": rollout.live.manifest_id(),
        "maxActions": rollout.max_actions,
        "policyVersion": "one-step-novelty-v1",
        "probed": {
            "actionKinds": rollout.probed_action_kinds.iter().cloned().collect::<Vec<_>>(),
            "eventTypes": rollout.probed_event_types.iter().cloned().collect::<Vec<_>>(),
        },
        "replayVerified": replay_verified,
        "rulesCoverage": "unranked_partial_rules",
        "schemaVersion": 1,
        "seed": manifest_seed(&rollout.live)?,
        "tooWide": rollout
            .too_wide
            .iter()
            .map(|entry| json!({
                "decisionIndex": entry.decision_index,
                "stateHash": entry.state_hash,
                "transcriptLength": entry.transcript_length,
            }))
            .collect::<Vec<_>>(),
        "transcriptHash": rollout.live.transcript_hash()?,
    });
    if let Value::Object(extra) = extra {
        for (key, value) in extra {
            result[key] = value;
        }
    }
    Ok(result)
}

fn first_seen_value(position: &NoveltyPosition, value: &Value) -> Value {
    json!({
        "firstSeen": {
            "decisionIndex": position.decision_index,
            "stateHash": position.state_hash,
            "transcriptLength": position.transcript_length,
        },
        "value": value,
    })
}

fn position(session: &Session, decision_index: u64) -> Result<NoveltyPosition, SessionError> {
    Ok(NoveltyPosition {
        decision_index,
        state_hash: session.state_hash()?,
        transcript_length: u64::try_from(session.transcript().len())
            .map_err(|_| SessionError::SequenceExhausted)?,
    })
}

fn action_kind(action: &LegalAction) -> Result<String, SessionError> {
    match action.descriptor.get("kind") {
        Some(Value::String(kind)) => Ok(kind.clone()),
        _ => Err(SessionError::Novelty {
            message: "novelty rollout action omitted kind".to_owned(),
        }),
    }
}

fn signal_key(signal: &Signal) -> String {
    format!("{}\0{}", signal.kind, signal.value)
}

fn preferred_frontier(left: &RankedFrontier, right: &RankedFrontier) -> Ordering {
    right
        .new_event_count
        .cmp(&left.new_event_count)
        .then_with(|| right.new_action_kind.cmp(&left.new_action_kind))
        .then_with(|| right.selected_by_fallback.cmp(&left.selected_by_fallback))
        .then_with(|| left.legal_index.cmp(&right.legal_index))
        .then_with(|| left.decision_index.cmp(&right.decision_index))
        .then_with(|| left.action_id.as_str().cmp(right.action_id.as_str()))
}

fn terminal_value(session: &Session) -> Result<Value, SessionError> {
    Ok(session.replay_value()?["state"]["terminal"].clone())
}

fn manifest_seed(session: &Session) -> Result<u64, SessionError> {
    let manifest: Value = serde_json::from_str(session.manifest_json())?;
    manifest
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| SessionError::Novelty {
            message: "novelty rollout manifest omitted seed".to_owned(),
        })
}

fn novelty_error(error: &impl ToString) -> SessionError {
    SessionError::Novelty {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{NOVELTY_ROLLOUT_ACTION_LIMIT, run_novelty_rollout};
    use crate::session::Session;
    use crate::synthetic::synthetic_demo_manifest_json;

    #[test]
    fn zero_horizon_from_synthetic_opening_is_resumable() {
        let session =
            Session::new(&synthetic_demo_manifest_json(31).expect("manifest")).expect("session");
        let output = run_novelty_rollout(&session, 0).expect("rollout");
        assert_eq!(output.result()["status"], "horizon");
        assert_eq!(output.result()["acceptedActionCount"], 0);
        assert_eq!(output.result()["replayVerified"], true);
        assert_eq!(output.emitted_checkpoints().len(), 1);
        assert_eq!(
            output.result()["checkpointId"],
            serde_json::json!(output.emitted_checkpoints()[0].checkpoint_id)
        );
    }

    #[test]
    fn rejects_an_oversize_horizon() {
        let session =
            Session::new(&synthetic_demo_manifest_json(31).expect("manifest")).expect("session");
        let error =
            run_novelty_rollout(&session, NOVELTY_ROLLOUT_ACTION_LIMIT + 1).expect_err("oversize");
        assert!(error.to_string().contains("maxActions"));
    }
}
