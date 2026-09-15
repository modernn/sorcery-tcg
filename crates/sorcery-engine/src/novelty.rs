//! Coverage-guided one-step novelty search owned by the engine.
//!
//! The checkpoint sink stays at the TypeScript boundary. This module returns
//! the emissions in order so a rejecting sink can be turned into the same
//! `checkpoint-failure` result the previous loop produced.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde_json::{Value, json};

use crate::canonical::{CanonicalError, canonical_json, identity_hash};
use crate::checkpoint::{CheckpointError, create_game_checkpoint};
use crate::contract::{ActionRequest, LegalAction};
use crate::session::{Session, SessionError, StepResult};

/// Maximum accepted novelty actions, matching the previous TypeScript bound.
pub const NOVELTY_ACTION_LIMIT: u64 = 500;
/// Width above which only the fallback action is probed.
pub const NOVELTY_WIDTH_LIMIT: usize = 128;

/// One checkpoint the caller should offer to its sink, plus the failure that
/// replaces the rollout if that sink rejects it.
#[derive(Clone, Debug)]
pub struct NoveltyEmission {
    /// Canonical game checkpoint.
    pub checkpoint: Value,
    /// Rollout result if the sink rejects this emission.
    pub failure_if_sink_rejects: Value,
}

/// Native novelty report and the ordered checkpoint emissions that produced it.
#[derive(Clone, Debug)]
pub struct NoveltyRollout {
    /// Emissions in the order a sink must observe them.
    pub emissions: Vec<NoveltyEmission>,
    /// Result when every sink accepts its checkpoint.
    pub result: Value,
}

/// Novelty search could not be started.
#[derive(Debug)]
pub enum NoveltyError {
    /// `maxActions` was not an integer in `0..=500`.
    InvalidLimit,
    /// Checkpoint, replay, or canonical hashing failed.
    Checkpoint(CheckpointError),
    /// Session transition failed.
    Session(SessionError),
    /// Canonical JSON failed.
    Canonical(CanonicalError),
    /// A boundary value could not be serialized.
    Json(serde_json::Error),
}

impl From<CheckpointError> for NoveltyError {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

impl From<SessionError> for NoveltyError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<CanonicalError> for NoveltyError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl fmt::Display for NoveltyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => formatter.write_str("noveltyRollout maxActions must be 0-500"),
            Self::Checkpoint(error) => write!(formatter, "{error}"),
            Self::Session(error) => write!(formatter, "{error}"),
            Self::Canonical(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<serde_json::Error> for NoveltyError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

struct FirstSeen {
    first_seen: Value,
    value: Value,
}

struct Probe {
    action: LegalAction,
    event_types: Vec<String>,
    index: usize,
    new_action_kind: bool,
    new_event_count: usize,
    selected_by_fallback: bool,
    state_hash: String,
}

#[derive(Clone)]
struct RankedFrontier {
    candidate: Value,
    decision_index: u64,
    legal_index: usize,
    new_action_kind: bool,
    new_event_count: usize,
    selected_by_fallback: bool,
}

/// Runs one novelty rollout from `session`.
///
/// # Errors
///
/// Returns [`NoveltyError`] when the limit, checkpoint, or session boundary fails.
pub fn run_novelty_rollout(
    session: &Session,
    max_actions: u64,
) -> Result<NoveltyRollout, NoveltyError> {
    if max_actions > NOVELTY_ACTION_LIMIT {
        return Err(NoveltyError::InvalidLimit);
    }
    let mut search = Search::new(session, max_actions)?;
    search.run()
}

struct Search {
    accepted: u64,
    branch_factors: Vec<FirstSeen>,
    committed_events: Vec<FirstSeen>,
    committed_kinds: Vec<FirstSeen>,
    emissions: Vec<NoveltyEmission>,
    emitted: HashSet<String>,
    frontier: HashMap<String, RankedFrontier>,
    initial_hash: String,
    manifest_id: String,
    max_actions: u64,
    offered_kinds: Vec<FirstSeen>,
    probed_events: HashSet<String>,
    probed_kinds: HashSet<String>,
    seed: u64,
    session: Session,
    too_wide: Vec<Value>,
}

impl Search {
    fn new(session: &Session, max_actions: u64) -> Result<Self, NoveltyError> {
        let replay = session.replay_value()?;
        Ok(Self {
            accepted: 0,
            branch_factors: Vec::new(),
            committed_events: Vec::new(),
            committed_kinds: Vec::new(),
            emissions: Vec::new(),
            emitted: HashSet::new(),
            frontier: HashMap::new(),
            initial_hash: hash_state(&replay["state"])?,
            manifest_id: session.manifest_id().to_string(),
            max_actions,
            offered_kinds: Vec::new(),
            probed_events: HashSet::new(),
            probed_kinds: HashSet::new(),
            seed: manifest_seed(session),
            session: session.clone(),
            too_wide: Vec::new(),
        })
    }

    fn run(&mut self) -> Result<NoveltyRollout, NoveltyError> {
        if !self.prepare(&self.session.clone())? {
            return Ok(self.failed(json!({ "kind": "replay-mismatch" }), false));
        }
        if !same_replay(&self.session, &self.session)? || !self.session.verify_replay()? {
            return Ok(self.failed(json!({ "kind": "replay-mismatch" }), false));
        }
        loop {
            let replay = self.session.replay_value()?;
            if replay["state"]["terminal"]["status"] == "finished" {
                return Ok(NoveltyRollout {
                    emissions: self.emissions.clone(),
                    result: self.summary(
                        &replay,
                        true,
                        json!({
                            "status": "completed",
                            "terminal": replay["state"]["terminal"].clone(),
                        }),
                    )?,
                });
            }
            if !self.prepare(&self.session.clone())? {
                return Ok(self.checkpoint_failure(true));
            }
            if self.accepted >= self.max_actions {
                return self.horizon(&replay);
            }
            if let Some(done) = self.advance(&replay)? {
                return Ok(done);
            }
        }
    }

    fn advance(&mut self, replay: &Value) -> Result<Option<NoveltyRollout>, NoveltyError> {
        let position = position_value(replay, self.accepted)?;
        let actions = self.session.legal_actions()?;
        self.remember_branch(actions.len(), &position);
        if actions.is_empty() {
            return Ok(Some(self.failed(json!({ "kind": "deadlock" }), true)));
        }
        for kind in unique_kinds(&actions) {
            self.remember_offered(&kind, &position);
        }
        let Some(fallback_index) = select_index(replay, &actions) else {
            return Ok(Some(self.failed(
                json!({ "kind": "exception", "phase": "selector" }),
                true,
            )));
        };
        let fallback = actions[fallback_index].clone();
        let selected = if actions.len() > NOVELTY_WIDTH_LIMIT {
            self.too_wide.push(position.clone());
            match self.probe_one(&fallback, fallback_index)? {
                ProbeOutcome::Selected(probe) => probe,
                ProbeOutcome::Failed(result) => return Ok(Some(result)),
            }
        } else {
            match self.probe_all(&actions, &fallback, &position)? {
                ProbeOutcome::Selected(probe) => probe,
                ProbeOutcome::Failed(result) => return Ok(Some(result)),
            }
        };
        self.probed_kinds.insert(action_kind(&selected.action));
        for event_type in &selected.event_types {
            self.probed_events.insert(event_type.clone());
        }
        let mut applied = self.session.clone();
        let stepped = applied.step(request_for(&selected.action))?;
        let StepResult::Accepted(_) = stepped else {
            return Ok(Some(
                self.failed(json!({ "kind": "replay-mismatch" }), false),
            ));
        };
        let probe_session = stepped_session(&self.session, &selected.action)?;
        if !same_replay(&probe_session, &applied)? {
            return Ok(Some(
                self.failed(json!({ "kind": "replay-mismatch" }), false),
            ));
        }
        self.remember_committed(&selected, &position);
        self.session = applied;
        self.accepted += 1;
        Ok(None)
    }

    fn probe_one(
        &mut self,
        action: &LegalAction,
        index: usize,
    ) -> Result<ProbeOutcome, NoveltyError> {
        let committed_kinds = committed_kind_set(&self.committed_kinds);
        let Ok(stepped) = self.speculate(action) else {
            return Ok(ProbeOutcome::Failed(self.failed(
                json!({
                    "actionId": action.action_id.to_string(),
                    "kind": "exception",
                    "phase": "fallback",
                }),
                true,
            )));
        };
        let StepResult::Accepted(receipt) = stepped else {
            return Ok(ProbeOutcome::Failed(self.rejected(action, "fallback")));
        };
        let event_types = event_types(&receipt.events);
        let state_hash =
            hash_state(&stepped_session(&self.session, action)?.replay_value()?["state"])?;
        Ok(ProbeOutcome::Selected(Probe {
            action: action.clone(),
            event_types: event_types.clone(),
            index,
            new_action_kind: !committed_kinds.contains(&action_kind(action)),
            new_event_count: event_types
                .iter()
                .filter(|event_type| !self.has_committed_event(event_type))
                .count(),
            selected_by_fallback: true,
            state_hash,
        }))
    }

    fn speculate(&self, action: &LegalAction) -> Result<StepResult, SessionError> {
        let mut probe = self.session.clone();
        probe.step(request_for(action))
    }

    fn probe_all(
        &mut self,
        actions: &[LegalAction],
        fallback: &LegalAction,
        position: &Value,
    ) -> Result<ProbeOutcome, NoveltyError> {
        let mut probes = Vec::new();
        for (index, action) in actions.iter().enumerate() {
            let Ok(stepped) = self.speculate(action) else {
                return Ok(ProbeOutcome::Failed(self.failed(
                    json!({
                        "actionId": action.action_id.to_string(),
                        "kind": "exception",
                        "phase": "probe",
                    }),
                    true,
                )));
            };
            let StepResult::Accepted(receipt) = stepped else {
                return Ok(ProbeOutcome::Failed(self.rejected(action, "probe")));
            };
            let event_types = event_types(&receipt.events);
            self.probed_kinds.insert(action_kind(action));
            for event_type in &event_types {
                self.probed_events.insert(event_type.clone());
            }
            probes.push(Probe {
                action: action.clone(),
                event_types,
                index,
                new_action_kind: !committed_kind_set(&self.committed_kinds)
                    .contains(&action_kind(action)),
                new_event_count: 0,
                selected_by_fallback: action.action_id == fallback.action_id,
                state_hash: hash_state(
                    &stepped_session(&self.session, action)?.replay_value()?["state"],
                )?,
            });
            let last = probes.len() - 1;
            probes[last].new_event_count = probes[last]
                .event_types
                .iter()
                .filter(|event_type| !self.has_committed_event(event_type))
                .count();
        }
        let selected = probes
            .iter()
            .enumerate()
            .reduce(|best, next| {
                if prefer_probe(&probes[next.0], &probes[best.0]) {
                    next
                } else {
                    best
                }
            })
            .map(|(index, _)| index)
            .ok_or(NoveltyError::InvalidLimit)?;
        self.record_frontier(&probes, selected, position)?;
        Ok(ProbeOutcome::Selected(probes.swap_remove(selected)))
    }

    fn record_frontier(
        &mut self,
        probes: &[Probe],
        selected: usize,
        position: &Value,
    ) -> Result<(), NoveltyError> {
        let selected_probe = &probes[selected];
        let mut committed_kinds = committed_kind_set(&self.committed_kinds);
        committed_kinds.insert(action_kind(&selected_probe.action));
        let mut committed_events = committed_event_set(&self.committed_events);
        for event_type in &selected_probe.event_types {
            committed_events.insert(event_type.clone());
        }
        let mut pending = Vec::new();
        for (index, probe) in probes.iter().enumerate() {
            if index == selected {
                continue;
            }
            let kind = action_kind(&probe.action);
            if !committed_kinds.contains(&kind) {
                pending.push((probe, "action-kind", kind.clone()));
            }
            for event_type in &probe.event_types {
                if !committed_events.contains(event_type) {
                    pending.push((probe, "event-type", event_type.clone()));
                }
            }
        }
        if pending.is_empty() {
            return Ok(());
        }
        let Some(checkpoint) = self.capture()? else {
            return Ok(());
        };
        let decision_index = self.accepted;
        for (probe, signal_kind, signal_value) in pending {
            let key = signal_key(signal_kind, &signal_value);
            let ranked = RankedFrontier {
                candidate: json!({
                    "actionId": probe.action.action_id.to_string(),
                    "actionKind": action_kind(&probe.action),
                    "checkpointId": checkpoint["checkpointId"].clone(),
                    "predictedEventTypes": probe.event_types,
                    "predictedStateHash": probe.state_hash,
                    "signal": { "kind": signal_kind, "value": signal_value },
                }),
                decision_index,
                legal_index: probe.index,
                new_action_kind: probe.new_action_kind,
                new_event_count: probe.new_event_count,
                selected_by_fallback: probe.selected_by_fallback,
            };
            self.frontier
                .entry(key)
                .and_modify(|existing| {
                    if prefer_frontier(&ranked, existing) {
                        *existing = ranked.clone();
                    }
                })
                .or_insert(ranked);
        }
        let _ = position;
        Ok(())
    }

    fn horizon(&mut self, replay: &Value) -> Result<NoveltyRollout, NoveltyError> {
        let Some(checkpoint) = self.capture()? else {
            return Ok(self.checkpoint_failure(true));
        };
        let checkpoint_id = checkpoint["checkpointId"].clone();
        Ok(NoveltyRollout {
            emissions: self.emissions.clone(),
            result: self.summary(
                replay,
                true,
                json!({
                    "checkpointId": checkpoint_id,
                    "status": "horizon",
                }),
            )?,
        })
    }

    fn capture(&mut self) -> Result<Option<Value>, NoveltyError> {
        let checkpoint = serde_json::to_value(create_game_checkpoint(&self.session)?)?;
        let id = checkpoint["checkpointId"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if self.emitted.insert(id) {
            let failure = self.checkpoint_failure_value(true)?;
            self.emissions.push(NoveltyEmission {
                checkpoint: checkpoint.clone(),
                failure_if_sink_rejects: failure,
            });
        }
        Ok(Some(checkpoint))
    }

    fn checkpoint_failure(&self, replay_verified: bool) -> NoveltyRollout {
        NoveltyRollout {
            emissions: self.emissions.clone(),
            result: self
                .checkpoint_failure_value(replay_verified)
                .unwrap_or_else(|_| json!({ "status": "failed" })),
        }
    }

    fn checkpoint_failure_value(&self, replay_verified: bool) -> Result<Value, NoveltyError> {
        let replay = self.session.replay_value()?;
        self.summary(
            &replay,
            replay_verified,
            json!({
                "failure": { "kind": "checkpoint-failure" },
                "status": "failed",
            }),
        )
    }

    fn failed(&mut self, reason: Value, replay_verified: bool) -> NoveltyRollout {
        if self.capture().ok().flatten().is_none() {
            return self.checkpoint_failure(reason["kind"] != "replay-mismatch");
        }
        let checkpoint_id = self
            .emissions
            .last()
            .and_then(|emission| emission.checkpoint.get("checkpointId"))
            .cloned()
            .unwrap_or(Value::Null);
        let mut failure = reason;
        if let Some(object) = failure.as_object_mut() {
            object.insert("checkpointId".to_owned(), checkpoint_id);
        }
        let replay = self.session.replay_value().ok();
        let result = replay.as_ref().map_or_else(
            || json!({ "status": "failed" }),
            |replay| {
                self.summary(
                    replay,
                    replay_verified,
                    json!({
                        "failure": failure,
                        "status": "failed",
                    }),
                )
                .unwrap_or_else(|_| json!({ "status": "failed" }))
            },
        );
        NoveltyRollout {
            emissions: self.emissions.clone(),
            result,
        }
    }

    fn rejected(&mut self, action: &LegalAction, phase: &str) -> NoveltyRollout {
        self.failed(
            json!({
                "actionId": action.action_id.to_string(),
                "code": rejection_code_name(phase),
                "kind": "engine-rejection",
                "phase": phase,
            }),
            true,
        )
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "callers build the extra object inline"
    )]
    fn summary(
        &self,
        replay: &Value,
        replay_verified: bool,
        extra: Value,
    ) -> Result<Value, NoveltyError> {
        let mut frontier = self.frontier.values().collect::<Vec<_>>();
        frontier.sort_by_key(|ranked| signal_of(&ranked.candidate));
        let mut value = json!({
            "acceptedActionCount": self.accepted,
            "classification": "authority-private",
            "coverage": {
                "branchFactors": first_seen_values(&self.branch_factors),
                "committedActionKinds": first_seen_values(&self.committed_kinds),
                "committedEventTypes": first_seen_values(&self.committed_events),
                "offeredActionKinds": first_seen_values(&self.offered_kinds),
            },
            "finalStateHash": hash_state(&replay["state"])?,
            "frontier": frontier.into_iter().map(|ranked| ranked.candidate.clone()).collect::<Vec<_>>(),
            "initialStateHash": self.initial_hash,
            "manifestId": self.manifest_id,
            "maxActions": self.max_actions,
            "policyVersion": "one-step-novelty-v1",
            "probed": {
                "actionKinds": sorted_set(&self.probed_kinds),
                "eventTypes": sorted_set(&self.probed_events),
            },
            "replayVerified": replay_verified,
            "rulesCoverage": "unranked_partial_rules",
            "schemaVersion": 1,
            "seed": self.seed,
            "tooWide": self.too_wide,
            "transcriptHash": identity_hash(&replay["transcript"])?.to_string(),
        });
        if let (Some(object), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
            for (key, item) in extra {
                object.insert(key.clone(), item.clone());
            }
        }
        Ok(value)
    }

    fn remember_branch(&mut self, width: usize, position: &Value) {
        if self
            .branch_factors
            .iter()
            .any(|seen| seen.value == json!(width))
        {
            return;
        }
        self.branch_factors.push(FirstSeen {
            first_seen: position.clone(),
            value: json!(width),
        });
    }

    fn remember_offered(&mut self, kind: &str, position: &Value) {
        if self
            .offered_kinds
            .iter()
            .any(|seen| seen.value == json!(kind))
        {
            return;
        }
        self.offered_kinds.push(FirstSeen {
            first_seen: position.clone(),
            value: json!(kind),
        });
    }

    fn remember_committed(&mut self, selected: &Probe, position: &Value) {
        let kind = action_kind(&selected.action);
        if !committed_kind_set(&self.committed_kinds).contains(&kind) {
            self.committed_kinds.push(FirstSeen {
                first_seen: position.clone(),
                value: json!(kind),
            });
        }
        for event_type in &selected.event_types {
            if !self.has_committed_event(event_type) {
                self.committed_events.push(FirstSeen {
                    first_seen: position.clone(),
                    value: json!(event_type),
                });
            }
            self.frontier.remove(&signal_key("event-type", event_type));
        }
        self.frontier.remove(&signal_key("action-kind", &kind));
    }

    fn has_committed_event(&self, event_type: &str) -> bool {
        committed_event_set(&self.committed_events).contains(event_type)
    }

    fn prepare(&self, _session: &Session) -> Result<bool, NoveltyError> {
        create_game_checkpoint(&self.session)?;
        Ok(true)
    }
}

enum ProbeOutcome {
    Failed(NoveltyRollout),
    Selected(Probe),
}

fn stepped_session(session: &Session, action: &LegalAction) -> Result<Session, SessionError> {
    let mut probe = session.clone();
    probe.step(request_for(action))?;
    Ok(probe)
}

fn rejection_code_name(phase: &str) -> &'static str {
    let _ = phase;
    "unknown_action"
}

pub(crate) fn request_for(action: &LegalAction) -> ActionRequest {
    ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }
}

fn action_kind(action: &LegalAction) -> String {
    action.descriptor["kind"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn unique_kinds(actions: &[LegalAction]) -> Vec<String> {
    let mut seen = HashSet::new();
    actions
        .iter()
        .map(action_kind)
        .filter(|kind| seen.insert(kind.clone()))
        .collect()
}

fn event_types(events: &[crate::contract::Event]) -> Vec<String> {
    let mut types = events
        .iter()
        .map(|event| event.event_type.clone())
        .collect::<Vec<_>>();
    types.sort();
    types.dedup();
    types
}

fn prefer_probe(left: &Probe, right: &Probe) -> bool {
    if left.new_event_count != right.new_event_count {
        return left.new_event_count > right.new_event_count;
    }
    if left.new_action_kind != right.new_action_kind {
        return left.new_action_kind;
    }
    if left.selected_by_fallback != right.selected_by_fallback {
        return left.selected_by_fallback;
    }
    left.index < right.index
}

fn prefer_frontier(left: &RankedFrontier, right: &RankedFrontier) -> bool {
    if left.new_event_count != right.new_event_count {
        return left.new_event_count > right.new_event_count;
    }
    if left.new_action_kind != right.new_action_kind {
        return left.new_action_kind;
    }
    if left.selected_by_fallback != right.selected_by_fallback {
        return left.selected_by_fallback;
    }
    if left.legal_index != right.legal_index {
        return left.legal_index < right.legal_index;
    }
    if left.decision_index != right.decision_index {
        return left.decision_index < right.decision_index;
    }
    signal_action_id(&left.candidate) <= signal_action_id(&right.candidate)
}

fn signal_of(candidate: &Value) -> String {
    let kind = candidate["signal"]["kind"].as_str().unwrap_or_default();
    let value = candidate["signal"]["value"].as_str().unwrap_or_default();
    signal_key(kind, value)
}

fn signal_action_id(candidate: &Value) -> String {
    candidate["actionId"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn signal_key(kind: &str, value: &str) -> String {
    format!("{kind}\0{value}")
}

fn first_seen_values(items: &[FirstSeen]) -> Vec<Value> {
    items
        .iter()
        .map(|item| json!({ "firstSeen": item.first_seen, "value": item.value }))
        .collect()
}

fn sorted_set(values: &HashSet<String>) -> Vec<&String> {
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn committed_kind_set(items: &[FirstSeen]) -> HashSet<String> {
    items
        .iter()
        .filter_map(|item| item.value.as_str().map(str::to_owned))
        .collect()
}

fn committed_event_set(items: &[FirstSeen]) -> HashSet<String> {
    committed_kind_set(items)
}

pub(crate) fn hash_state(state: &Value) -> Result<String, NoveltyError> {
    Ok(identity_hash(state)?.to_string())
}

fn position_value(replay: &Value, decision_index: u64) -> Result<Value, NoveltyError> {
    Ok(json!({
        "decisionIndex": decision_index,
        "stateHash": hash_state(&replay["state"])?,
        "transcriptLength": replay["transcript"].as_array().map_or(0, Vec::len),
    }))
}

fn same_replay(left: &Session, right: &Session) -> Result<bool, NoveltyError> {
    Ok(canonical_json(&left.replay_value()?)? == canonical_json(&right.replay_value()?)?)
}

fn manifest_seed(session: &Session) -> u64 {
    serde_json::from_str::<Value>(session.manifest_json())
        .ok()
        .and_then(|manifest| manifest["seed"].as_u64())
        .unwrap_or(0)
}

pub(crate) fn select_index(replay: &Value, actions: &[LegalAction]) -> Option<usize> {
    let state = &replay["state"];
    let seat = state["decisionSeat"].as_str().unwrap_or("north");
    let enemy = if seat == "north" { "south" } else { "north" };
    let player = &state["players"][seat];
    let atlas_len = player["atlas"].as_array().map_or(0, Vec::len);
    let spellbook_len = player["spellbook"].as_array().map_or(0, Vec::len);
    let draw_zone = if atlas_len > 3 || spellbook_len <= atlas_len {
        "atlas"
    } else {
        "spellbook"
    };
    let enemy_avatar = state["players"][enemy]["avatar"]["location"]
        .as_str()
        .unwrap_or("");
    let movement = best_movement(actions, enemy_avatar);
    let tactic = tactic_index(state, actions, seat, enemy, movement);
    let powered = powered_movement(state, actions, seat, movement);
    find_keep(actions)
        .or_else(|| find_kind(actions, "play-site"))
        .or_else(|| find_kind(actions, "summon-minion"))
        .or_else(|| find_draw(actions, draw_zone))
        .or(powered)
        .or(tactic)
        .or(finite_movement(actions, movement))
        .or_else(|| find_kind(actions, "end-turn"))
        .or(if actions.is_empty() { None } else { Some(0) })
}

fn find_keep(actions: &[LegalAction]) -> Option<usize> {
    actions.iter().position(|action| {
        action.descriptor["kind"] == "mulligan"
            && action.descriptor["atlasOrder"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && action.descriptor["spellbookOrder"]
                .as_array()
                .is_some_and(Vec::is_empty)
    })
}

fn find_kind(actions: &[LegalAction], kind: &str) -> Option<usize> {
    actions
        .iter()
        .position(|action| action.descriptor["kind"] == kind)
}

fn find_draw(actions: &[LegalAction], zone: &str) -> Option<usize> {
    actions
        .iter()
        .position(|action| action.descriptor["kind"] == "draw" && action.descriptor["zone"] == zone)
}

fn best_movement(actions: &[LegalAction], enemy_avatar: &str) -> Option<(usize, i64)> {
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            if action.descriptor["kind"] != "move-and-attack" {
                return None;
            }
            let path_len = action.descriptor["path"].as_array().map_or(0, Vec::len);
            let to_cell = action.descriptor["to"]["cell"].as_str().unwrap_or("");
            let to_region = action.descriptor["to"]["region"].as_str().unwrap_or("");
            let distance = if path_len == 1 && to_cell == enemy_avatar && to_region == "surface" {
                -1
            } else if path_len > 1 {
                cell_distance(to_cell, enemy_avatar)
            } else {
                i64::MAX
            };
            Some((index, distance))
        })
        .min_by_key(|(_, distance)| *distance)
}

fn finite_movement(actions: &[LegalAction], movement: Option<(usize, i64)>) -> Option<usize> {
    let _ = actions;
    movement.and_then(|(index, distance)| (distance != i64::MAX).then_some(index))
}

fn powered_movement(
    state: &Value,
    actions: &[LegalAction],
    seat: &str,
    movement: Option<(usize, i64)>,
) -> Option<usize> {
    let (index, _) = movement?;
    let unit_id = actions[index].descriptor["unitInstanceId"].as_str()?;
    let avatar_id = state["players"][seat]["avatar"]["card"]["instanceId"].as_str()?;
    let powered = if unit_id == avatar_id {
        state["players"][seat]["avatar"]["temporaryPowerSources"]
            .as_array()
            .is_some_and(|sources| !sources.is_empty())
    } else {
        state["realm"]["units"].as_array().is_some_and(|units| {
            units.iter().any(|unit| {
                unit["instanceId"] == unit_id
                    && unit["temporaryPowerSources"]
                        .as_array()
                        .is_some_and(|sources| !sources.is_empty())
            })
        })
    };
    powered.then_some(index)
}

fn tactic_index(
    state: &Value,
    actions: &[LegalAction],
    seat: &str,
    enemy: &str,
    movement: Option<(usize, i64)>,
) -> Option<usize> {
    actions.iter().position(|action| {
        let descriptor = &action.descriptor;
        match descriptor["kind"].as_str() {
            Some("cast-magic") => magic_tactic(state, descriptor, seat, enemy, movement, actions),
            Some("shoot-projectile") => descriptor["hit"]["seat"] == enemy,
            Some("shoot-drag-projectile") => {
                descriptor["hit"]["seat"] == enemy && descriptor["fightOnArrival"] != true
            }
            Some("activate-sparkmage") => sparkmage_tactic(state, descriptor, enemy),
            _ => false,
        }
    })
}

fn magic_tactic(
    state: &Value,
    descriptor: &Value,
    seat: &str,
    enemy: &str,
    movement: Option<(usize, i64)>,
    actions: &[LegalAction],
) -> bool {
    let Some(card_id) = descriptor["cardId"].as_str() else {
        return false;
    };
    let definition = &state["cards"][card_id];
    if definition["cardType"] != "magic" {
        return false;
    }
    if definition.get("damageTargetUnit").is_some() && descriptor["target"]["seat"] == enemy {
        return true;
    }
    if definition.get("grantPowerToAllyThisTurn").is_none() || descriptor["ally"]["seat"] != seat {
        return false;
    }
    let Some((index, _)) = movement else {
        return false;
    };
    let moving = &actions[index].descriptor;
    moving["kind"] == "move-and-attack"
        && moving["unitInstanceId"] == descriptor["ally"]["instanceId"]
        && moving["to"]["cell"] == state["players"][enemy]["avatar"]["location"]
        && moving["to"]["region"] == "surface"
}

fn sparkmage_tactic(state: &Value, descriptor: &Value, enemy: &str) -> bool {
    let seat = state["decisionSeat"].as_str().unwrap_or("north");
    if state["players"][seat]["airThresholdsCastThisTurn"]
        .as_u64()
        .unwrap_or(0)
        == 0
    {
        return false;
    }
    let source = descriptor["sourceInstanceId"].as_str().unwrap_or("");
    let cell = descriptor["targetLocation"]["cell"].as_str().unwrap_or("");
    let region = descriptor["targetLocation"]["region"]
        .as_str()
        .unwrap_or("");
    let mut controllers = Vec::new();
    for seat_name in ["north", "south"] {
        let avatar = &state["players"][seat_name]["avatar"];
        if avatar["card"]["instanceId"] != source
            && avatar["location"] == cell
            && avatar["region"] == region
        {
            controllers.push(seat_name);
        }
    }
    if let Some(units) = state["realm"]["units"].as_array() {
        for unit in units {
            if unit["instanceId"] != source
                && unit["location"] == cell
                && unit["region"] == region
                && let Some(controller) = unit["controller"].as_str()
            {
                controllers.push(controller);
            }
        }
    }
    !controllers.is_empty() && controllers.iter().all(|controller| *controller == enemy)
}

fn cell_distance(left: &str, right: &str) -> i64 {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    if left_chars.is_empty() || right_chars.is_empty() {
        return i64::MAX;
    }
    let left_row = i64::from(
        left_chars
            .get(1)
            .and_then(|ch| ch.to_digit(10))
            .unwrap_or(0),
    );
    let right_row = i64::from(
        right_chars
            .get(1)
            .and_then(|ch| ch.to_digit(10))
            .unwrap_or(0),
    );
    i64::from((left_chars[0] as u32).abs_diff(right_chars[0] as u32)) + (left_row - right_row).abs()
}
