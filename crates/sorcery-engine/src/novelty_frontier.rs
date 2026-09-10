//! Signal-guided bounded novelty search over checkpoint-rooted engine actions.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use crate::canonical::IdentityHash;
use crate::checkpoint::{GameCheckpoint, resume_game_checkpoint};
use crate::novelty_dispatch::{
    ForcedNoveltyInput, ForcedNoveltyOutput, run_novelty_from_forced_action,
};
use crate::novelty_rollout::{NOVELTY_ROLLOUT_ACTION_LIMIT, run_novelty_rollout};
use crate::session::{Session, SessionError};

/// Widest accepted frontier-branch expansion.
pub const NOVELTY_FRONTIER_BRANCH_LIMIT: usize = 32;
const ROOT_JOB_ID: &str = "root";

/// Public frontier-search report plus checkpoints emitted for TypeScript sinks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoveltyFrontierSearchOutput {
    emitted_checkpoints: Vec<GameCheckpoint>,
    result: Value,
}

impl NoveltyFrontierSearchOutput {
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
struct Signal {
    kind: String,
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrontierCandidate {
    action_id: IdentityHash,
    action_kind: String,
    checkpoint_id: IdentityHash,
    predicted_event_types: Vec<String>,
    predicted_state_hash: IdentityHash,
    signal: Signal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrontierSeed {
    action_id: IdentityHash,
    action_kind: String,
    branch_id: String,
    checkpoint_id: IdentityHash,
    depth: u64,
    parent_branch_id: Option<String>,
    parent_job_id: String,
    predicted_event_types: Vec<String>,
    predicted_state_hash: IdentityHash,
    signals: Vec<Signal>,
}

struct Search {
    checkpoints: HashMap<IdentityHash, GameCheckpoint>,
    emitted: Vec<GameCheckpoint>,
    exercised: HashSet<String>,
    max_actions: usize,
    max_branches: usize,
    next_branch_ordinal: u64,
    queue: Vec<FrontierSeed>,
    seen_branches: HashSet<String>,
}

/// Runs one-step novelty from `root`, then expands unchosen signals as forced branches.
///
/// The caller's session is not mutated. Coverage is shared across dispatched branches.
/// Expansion stops after [`NOVELTY_FRONTIER_BRANCH_LIMIT`] branches or the first failed
/// child rollout.
///
/// # Errors
///
/// Returns [`SessionError`] when a bound is invalid, a frontier prediction is inconsistent,
/// a captured checkpoint is missing, or a forced dispatch fails.
pub fn run_novelty_frontier_search(
    root: &Session,
    max_actions: usize,
    max_branches: usize,
) -> Result<NoveltyFrontierSearchOutput, SessionError> {
    if max_actions > NOVELTY_ROLLOUT_ACTION_LIMIT {
        return Err(search_error(&format!(
            "maxActions must be 0-{NOVELTY_ROLLOUT_ACTION_LIMIT}"
        )));
    }
    if max_branches > NOVELTY_FRONTIER_BRANCH_LIMIT {
        return Err(search_error(&format!(
            "maxBranches must be 0-{NOVELTY_FRONTIER_BRANCH_LIMIT}"
        )));
    }
    Search::new(max_actions, max_branches).run(root)
}

impl Search {
    fn new(max_actions: usize, max_branches: usize) -> Self {
        Self {
            checkpoints: HashMap::new(),
            emitted: Vec::new(),
            exercised: HashSet::new(),
            max_actions,
            max_branches,
            next_branch_ordinal: 1,
            queue: Vec::new(),
            seen_branches: HashSet::new(),
        }
    }

    fn run(mut self, root: &Session) -> Result<NoveltyFrontierSearchOutput, SessionError> {
        let root_output = run_novelty_rollout(root, self.max_actions)?;
        self.remember_all(root_output.emitted_checkpoints())?;
        self.enqueue_frontier(root_output.result(), None, 1)?;
        let (frontier_branches, cursor, mut pruned_covered) = self.expand_branches()?;
        let frontier_pending = self.collect_pending(cursor, &mut pruned_covered);
        self.finish(
            root_output.result(),
            &frontier_branches,
            &frontier_pending,
            pruned_covered,
        )
    }

    fn expand_branches(&mut self) -> Result<(Vec<Value>, usize, u64), SessionError> {
        let mut frontier_branches = Vec::new();
        let mut cursor = 0;
        let mut pruned_covered = 0_u64;
        let mut stop_after_failure = false;
        while cursor < self.queue.len()
            && frontier_branches.len() < self.max_branches
            && !stop_after_failure
        {
            let seed = self.queue[cursor].clone();
            cursor += 1;
            if let Some(branch) = self.dispatch_seed(&seed, &mut pruned_covered)? {
                let failed = branch["result"].get("status") == Some(&json!("failed"));
                if !failed {
                    self.enqueue_frontier(
                        &branch["result"],
                        Some(seed.branch_id.as_str()),
                        seed.depth + 1,
                    )?;
                }
                frontier_branches.push(branch);
                stop_after_failure = failed;
            }
        }
        Ok((frontier_branches, cursor, pruned_covered))
    }

    fn dispatch_seed(
        &mut self,
        seed: &FrontierSeed,
        pruned_covered: &mut u64,
    ) -> Result<Option<Value>, SessionError> {
        let novel = uncovered_signals(&seed.signals, &self.exercised);
        if novel.is_empty() {
            *pruned_covered += 1;
            return Ok(None);
        }
        let dispatched = self.dispatch(seed)?;
        if !signals_match_entry(seed, dispatched.entry().event_types()) {
            return Err(search_error(
                "novelty frontier prediction did not replay exactly",
            ));
        }
        self.mark_exercised(&seed.action_kind, dispatched.entry().event_types());
        Ok(Some(json!({
            "actionId": seed.action_id,
            "actionKind": seed.action_kind,
            "branchId": seed.branch_id,
            "checkpointId": seed.checkpoint_id,
            "depth": seed.depth,
            "entryActionCount": 1,
            "entryEventTypes": dispatched.entry().event_types(),
            "novelSignalsAtDispatch": novel.iter().map(signal_value).collect::<Vec<_>>(),
            "parentBranchId": seed.parent_branch_id,
            "parentJobId": seed.parent_job_id,
            "result": dispatched.result(),
            "signals": seed.signals.iter().map(signal_value).collect::<Vec<_>>(),
        })))
    }

    fn collect_pending(&self, cursor: usize, pruned_covered: &mut u64) -> Vec<Value> {
        let mut frontier_pending = Vec::new();
        for seed in self.queue.iter().skip(cursor) {
            let signals = uncovered_signals(&seed.signals, &self.exercised);
            if signals.is_empty() {
                *pruned_covered += 1;
                continue;
            }
            frontier_pending.push(json!({
                "actionId": seed.action_id,
                "actionKind": seed.action_kind,
                "branchId": seed.branch_id,
                "checkpointId": seed.checkpoint_id,
                "depth": seed.depth,
                "parentBranchId": seed.parent_branch_id,
                "parentJobId": seed.parent_job_id,
                "predictedEventTypes": seed.predicted_event_types,
                "predictedStateHash": seed.predicted_state_hash,
                "signals": signals.iter().map(signal_value).collect::<Vec<_>>(),
            }));
        }
        frontier_pending
    }

    fn finish(
        self,
        root: &Value,
        frontier_branches: &[Value],
        frontier_pending: &[Value],
        pruned_covered: u64,
    ) -> Result<NoveltyFrontierSearchOutput, SessionError> {
        self.require_reported_checkpoints(root)?;
        for branch in frontier_branches {
            self.require_reported_checkpoints(&branch["result"])?;
        }
        for pending in frontier_pending {
            self.require_checkpoint(&json_hash(pending, "checkpointId")?)?;
        }
        let pending_signals = pending_signal_count(frontier_pending);
        let frontier_max_depth = frontier_branches
            .iter()
            .filter_map(|branch| branch["depth"].as_u64())
            .max()
            .unwrap_or(0);
        Ok(NoveltyFrontierSearchOutput {
            emitted_checkpoints: self.emitted,
            result: json!({
                "classification": "authority-private",
                "frontierBranches": frontier_branches,
                "frontierPending": frontier_pending,
                "policyVersion": "signal-guided-bounded-frontier-v2",
                "root": root,
                "schemaVersion": 2,
                "totals": {
                    "branchLimit": self.max_branches,
                    "frontierBranches": frontier_branches.len(),
                    "frontierCompleted": count_status(frontier_branches, "completed"),
                    "frontierFailed": count_status(frontier_branches, "failed"),
                    "frontierHorizon": count_status(frontier_branches, "horizon"),
                    "frontierLimitReached": frontier_branches.len() == self.max_branches
                        && !frontier_pending.is_empty(),
                    "frontierMaxDepth": frontier_max_depth,
                    "frontierPending": frontier_pending.len(),
                    "frontierPendingSignals": pending_signals,
                    "frontierPrunedCovered": pruned_covered,
                    "rootStatus": root.get("status"),
                },
            }),
        })
    }

    fn enqueue_frontier(
        &mut self,
        result: &Value,
        parent_branch_id: Option<&str>,
        depth: u64,
    ) -> Result<(), SessionError> {
        let candidates = parse_frontier(result)?;
        let mut grouped: HashMap<String, FrontierSeed> = HashMap::new();
        for candidate in candidates {
            let key = branch_key(&candidate.checkpoint_id, &candidate.action_id);
            if let Some(existing) = grouped.get_mut(&key) {
                if existing.action_kind != candidate.action_kind
                    || existing.predicted_state_hash != candidate.predicted_state_hash
                    || existing.predicted_event_types != candidate.predicted_event_types
                {
                    return Err(search_error(
                        "novelty frontier action has inconsistent predictions",
                    ));
                }
                existing.signals.push(candidate.signal);
            } else {
                grouped.insert(
                    key,
                    FrontierSeed {
                        action_id: candidate.action_id,
                        action_kind: candidate.action_kind,
                        branch_id: String::new(),
                        checkpoint_id: candidate.checkpoint_id,
                        depth,
                        parent_branch_id: parent_branch_id.map(str::to_owned),
                        parent_job_id: ROOT_JOB_ID.to_owned(),
                        predicted_event_types: candidate.predicted_event_types,
                        predicted_state_hash: candidate.predicted_state_hash,
                        signals: vec![candidate.signal],
                    },
                );
            }
        }
        let mut seeds = grouped.into_values().collect::<Vec<_>>();
        seeds.sort_by(|left, right| {
            left.checkpoint_id
                .as_str()
                .cmp(right.checkpoint_id.as_str())
                .then_with(|| left.action_id.as_str().cmp(right.action_id.as_str()))
        });
        for mut seed in seeds {
            let key = branch_key(&seed.checkpoint_id, &seed.action_id);
            if !self.seen_branches.insert(key) {
                continue;
            }
            seed.signals.sort_by_key(signal_key);
            seed.branch_id = format!("{ROOT_JOB_ID}:branch-{}", self.next_branch_ordinal);
            self.next_branch_ordinal = self
                .next_branch_ordinal
                .checked_add(1)
                .ok_or(SessionError::SequenceExhausted)?;
            self.queue.push(seed);
        }
        Ok(())
    }

    fn dispatch(&mut self, seed: &FrontierSeed) -> Result<ForcedNoveltyOutput, SessionError> {
        let checkpoint = self
            .checkpoints
            .get(&seed.checkpoint_id)
            .ok_or_else(|| search_error("novelty frontier checkpoint was not captured"))?;
        let mut session =
            resume_game_checkpoint(checkpoint).map_err(|error| search_error(&error.to_string()))?;
        let dispatched = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: seed.action_id.as_str(),
                action_kind: &seed.action_kind,
                max_actions: self.max_actions,
                predicted_event_types: &seed.predicted_event_types,
                predicted_state_hash: seed.predicted_state_hash.as_str(),
            },
        )?;
        self.remember_all(dispatched.emitted_checkpoints())?;
        Ok(dispatched)
    }

    fn mark_exercised(&mut self, action_kind: &str, event_types: &[String]) {
        self.exercised.insert(signal_key(&Signal {
            kind: "action-kind".to_owned(),
            value: action_kind.to_owned(),
        }));
        for event_type in event_types {
            self.exercised.insert(signal_key(&Signal {
                kind: "event-type".to_owned(),
                value: event_type.clone(),
            }));
        }
    }

    fn remember_all(&mut self, checkpoints: &[GameCheckpoint]) -> Result<(), SessionError> {
        for checkpoint in checkpoints {
            self.remember(checkpoint.clone())?;
        }
        Ok(())
    }

    fn remember(&mut self, checkpoint: GameCheckpoint) -> Result<(), SessionError> {
        if let Some(existing) = self.checkpoints.get(&checkpoint.checkpoint_id) {
            if existing != &checkpoint {
                return Err(search_error(
                    "novelty frontier checkpoint identity collision",
                ));
            }
            return Ok(());
        }
        self.checkpoints
            .insert(checkpoint.checkpoint_id.clone(), checkpoint.clone());
        self.emitted.push(checkpoint);
        Ok(())
    }

    fn require_reported_checkpoints(&self, result: &Value) -> Result<(), SessionError> {
        for candidate in parse_frontier(result)? {
            self.require_checkpoint(&candidate.checkpoint_id)?;
        }
        let status = result.get("status").and_then(Value::as_str);
        if status == Some("horizon") {
            self.require_checkpoint(&json_hash(result, "checkpointId")?)?;
        }
        if status == Some("failed")
            && let Some(failure) = result.get("failure")
            && failure.get("checkpointId").is_some()
        {
            self.require_checkpoint(&json_hash(failure, "checkpointId")?)?;
        }
        Ok(())
    }

    fn require_checkpoint(&self, checkpoint_id: &IdentityHash) -> Result<(), SessionError> {
        if self.checkpoints.contains_key(checkpoint_id) {
            Ok(())
        } else {
            Err(search_error(
                "novelty frontier search did not capture a reported checkpoint",
            ))
        }
    }
}

fn parse_frontier(result: &Value) -> Result<Vec<FrontierCandidate>, SessionError> {
    let Some(items) = result.get("frontier").and_then(Value::as_array) else {
        return Err(search_error("novelty frontier omitted frontier"));
    };
    items.iter().map(parse_candidate).collect()
}

fn parse_candidate(value: &Value) -> Result<FrontierCandidate, SessionError> {
    let signal_object = value
        .get("signal")
        .ok_or_else(|| search_error("novelty frontier omitted signal"))?;
    let kind = json_string(signal_object, "kind")?;
    if kind != "action-kind" && kind != "event-type" {
        return Err(search_error("novelty frontier signal kind is unsupported"));
    }
    Ok(FrontierCandidate {
        action_id: json_hash(value, "actionId")?,
        action_kind: json_string(value, "actionKind")?,
        checkpoint_id: json_hash(value, "checkpointId")?,
        predicted_event_types: json_strings(value, "predictedEventTypes")?,
        predicted_state_hash: json_hash(value, "predictedStateHash")?,
        signal: Signal {
            kind,
            value: json_string(signal_object, "value")?,
        },
    })
}

fn json_string(value: &Value, key: &str) -> Result<String, SessionError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| search_error(&format!("novelty frontier omitted {key}")))
}

fn json_hash(value: &Value, key: &str) -> Result<IdentityHash, SessionError> {
    IdentityHash::parse(&json_string(value, key)?)
        .map_err(|_| search_error(&format!("novelty frontier {key} is invalid")))
}

fn json_strings(value: &Value, key: &str) -> Result<Vec<String>, SessionError> {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        return Err(search_error(&format!("novelty frontier omitted {key}")));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(str::to_owned).ok_or_else(|| {
                search_error(&format!(
                    "novelty frontier {key} must be an array of strings"
                ))
            })
        })
        .collect()
}

fn signal_key(signal: &Signal) -> String {
    format!("{}\0{}", signal.kind, signal.value)
}

fn signal_value(signal: &Signal) -> Value {
    json!({
        "kind": signal.kind,
        "value": signal.value,
    })
}

fn branch_key(checkpoint_id: &IdentityHash, action_id: &IdentityHash) -> String {
    format!("{}\0{}", checkpoint_id.as_str(), action_id.as_str())
}

fn count_status(branches: &[Value], status: &str) -> usize {
    branches
        .iter()
        .filter(|branch| branch["result"]["status"] == status)
        .count()
}

fn uncovered_signals(signals: &[Signal], exercised: &HashSet<String>) -> Vec<Signal> {
    signals
        .iter()
        .filter(|signal| !exercised.contains(&signal_key(signal)))
        .cloned()
        .collect()
}

fn signals_match_entry(seed: &FrontierSeed, event_types: &[String]) -> bool {
    seed.signals.iter().all(|signal| {
        if signal.kind == "action-kind" {
            signal.value == seed.action_kind
        } else {
            event_types
                .iter()
                .any(|event_type| event_type == &signal.value)
        }
    })
}

fn pending_signal_count(frontier_pending: &[Value]) -> usize {
    frontier_pending
        .iter()
        .flat_map(|pending| pending["signals"].as_array().cloned().unwrap_or_default())
        .map(|signal| {
            format!(
                "{}\0{}",
                signal["kind"].as_str().unwrap_or_default(),
                signal["value"].as_str().unwrap_or_default()
            )
        })
        .collect::<HashSet<_>>()
        .len()
}

fn search_error(message: &str) -> SessionError {
    SessionError::Novelty {
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{NOVELTY_FRONTIER_BRANCH_LIMIT, run_novelty_frontier_search};
    use crate::novelty_rollout::NOVELTY_ROLLOUT_ACTION_LIMIT;
    use crate::session::Session;
    use crate::synthetic::synthetic_demo_session;

    fn opening_session() -> Session {
        synthetic_demo_session(31)
    }

    #[test]
    fn zero_horizon_root_is_reproducible_without_branches() {
        let session = opening_session();
        let first = run_novelty_frontier_search(&session, 0, 0).expect("search");
        let second = run_novelty_frontier_search(&session, 0, 0).expect("search");
        assert_eq!(first.result(), second.result());
        assert_eq!(first.result()["root"]["status"], "horizon");
        assert_eq!(first.result()["root"]["acceptedActionCount"], 0);
        assert_eq!(
            first.result()["frontierBranches"].as_array().map(Vec::len),
            Some(0)
        );
        assert_eq!(first.result()["totals"]["branchLimit"], 0);
        assert_eq!(first.result()["totals"]["rootStatus"], "horizon");
        assert_eq!(first.emitted_checkpoints().len(), 1);
    }

    #[test]
    fn rejects_an_oversize_branch_bound() {
        let session = opening_session();
        let error = run_novelty_frontier_search(&session, 0, NOVELTY_FRONTIER_BRANCH_LIMIT + 1)
            .expect_err("oversize");
        assert!(error.to_string().contains("maxBranches"));
    }

    #[test]
    fn rejects_an_oversize_horizon() {
        let session = opening_session();
        let error = run_novelty_frontier_search(&session, NOVELTY_ROLLOUT_ACTION_LIMIT + 1, 0)
            .expect_err("oversize");
        assert!(error.to_string().contains("maxActions"));
    }
}
