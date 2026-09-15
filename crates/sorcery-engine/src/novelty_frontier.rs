//! Signal-guided bounded frontier scheduling over private novelty rollouts.
//!
//! Matches the previous TypeScript private novelty gauntlet loop: four lesson
//! jobs, FIFO frontier merge by checkpoint+action, prune covered signals, and
//! dispatch at most 32 branches before reporting pending work.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde_json::{Map, Value, json};

use crate::canonical::{CanonicalError, canonical_json};
use crate::checkpoint::{
    CheckpointError, create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use crate::contract::RejectionCode;
use crate::novelty::{self, NoveltyError, hash_state, request_for};
use crate::session::{Session, SessionError, StepResult};

/// Maximum frontier branches dispatched in one private novelty gauntlet.
pub const FRONTIER_BRANCH_LIMIT: usize = 32;
const POLICY_VERSION: &str = "signal-guided-bounded-frontier-v2";

/// One private lesson orientation to schedule.
#[derive(Clone, Debug)]
pub struct FrontierJob {
    /// Stable job identity, for example `air-vs-earth-lesson:original`.
    pub job_id: String,
    /// Pinned private lesson identifier.
    pub lesson_id: String,
    /// Seat orientation label (`original` or `swapped`).
    pub orientation: String,
    /// Canonical game manifest JSON for this orientation.
    pub manifest_json: String,
}

/// Native private novelty frontier report plus checkpoint sink payload.
#[derive(Clone, Debug)]
pub struct FrontierRun {
    /// Checkpoints in first-seen emission order for the TypeScript sink.
    pub checkpoints: Vec<(String, Value)>,
    /// Full gauntlet report (`savedCheckpoints` left at 0 for the sink to patch).
    pub report: Value,
}

/// Private novelty frontier scheduling failed.
#[derive(Debug)]
pub enum FrontierError {
    /// `maxActions` was outside `0..=500`.
    InvalidLimit,
    /// A frontier invariant or prediction check failed.
    Invalid(String),
    /// Novelty rollout failed to start.
    Novelty(NoveltyError),
    /// Checkpoint create/parse/resume failed.
    Checkpoint(CheckpointError),
    /// Session transition failed.
    Session(SessionError),
    /// Canonical JSON failed.
    Canonical(CanonicalError),
    /// Boundary JSON failed.
    Json(serde_json::Error),
}

impl From<NoveltyError> for FrontierError {
    fn from(error: NoveltyError) -> Self {
        match error {
            NoveltyError::InvalidLimit => Self::InvalidLimit,
            other => Self::Novelty(other),
        }
    }
}

impl From<CheckpointError> for FrontierError {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

impl From<SessionError> for FrontierError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<CanonicalError> for FrontierError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for FrontierError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl fmt::Display for FrontierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => formatter.write_str("novelty frontier maxActions must be 0-500"),
            Self::Invalid(message) => formatter.write_str(message),
            Self::Novelty(error) => write!(formatter, "{error}"),
            Self::Checkpoint(error) => write!(formatter, "{error}"),
            Self::Session(error) => write!(formatter, "{error}"),
            Self::Canonical(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
        }
    }
}

#[derive(Clone)]
struct Signal {
    kind: String,
    value: String,
}

#[derive(Clone)]
struct FrontierSeed {
    action_id: String,
    action_kind: String,
    branch_id: String,
    checkpoint_id: String,
    depth: u64,
    parent_branch_id: Option<String>,
    parent_job_id: String,
    predicted_event_types: Value,
    predicted_state_hash: String,
    signals: Vec<Signal>,
}

struct CheckpointStore {
    by_id: HashMap<String, Value>,
    order: Vec<String>,
}

impl CheckpointStore {
    fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn capture(&mut self, checkpoint: &Value) -> Result<(), CaptureOutcome> {
        let id = checkpoint["checkpointId"]
            .as_str()
            .ok_or_else(|| {
                CaptureOutcome::Error(FrontierError::Invalid(
                    "private novelty checkpoint ID is missing".to_owned(),
                ))
            })?
            .to_owned();
        let incoming = checkpoint_bytes(checkpoint).map_err(CaptureOutcome::Error)?;
        if let Some(existing) = self.by_id.get(&id) {
            let previous = checkpoint_bytes(existing).map_err(CaptureOutcome::Error)?;
            if previous != incoming {
                return Err(CaptureOutcome::Collision);
            }
            return Ok(());
        }
        self.by_id.insert(id.clone(), checkpoint.clone());
        self.order.push(id);
        Ok(())
    }

    fn get(&self, checkpoint_id: &str) -> Option<&Value> {
        self.by_id.get(checkpoint_id)
    }

    fn contains(&self, checkpoint_id: &str) -> bool {
        self.by_id.contains_key(checkpoint_id)
    }

    fn into_ordered(self) -> Vec<(String, Value)> {
        let mut by_id = self.by_id;
        self.order
            .into_iter()
            .filter_map(|id| by_id.remove(&id).map(|checkpoint| (id, checkpoint)))
            .collect()
    }
}

enum CaptureOutcome {
    Collision,
    Error(FrontierError),
}

/// Runs the private novelty jobs and bounded signal-guided frontier.
///
/// # Errors
///
/// Returns [`FrontierError`] when limits, predictions, checkpoints, or sessions fail.
pub fn run_private_novelty_frontier(
    jobs: &[FrontierJob],
    max_actions: u64,
) -> Result<FrontierRun, FrontierError> {
    if max_actions > novelty::NOVELTY_ACTION_LIMIT {
        return Err(FrontierError::InvalidLimit);
    }
    if jobs.len() != 4 {
        return Err(FrontierError::Invalid(
            "private novelty frontier requires exactly 4 jobs".to_owned(),
        ));
    }

    let mut store = CheckpointStore::new();
    let job_rows = run_jobs(jobs, max_actions, &mut store)?;
    let mut queue = Vec::new();
    let mut seen_branches = HashSet::new();
    let mut next_branch_ordinal = 1;
    for row in &job_rows {
        enqueue_frontier(
            &mut queue,
            &mut seen_branches,
            &mut next_branch_ordinal,
            &row["result"]["frontier"],
            row["jobId"].as_str().unwrap_or_default(),
            None,
            1,
        )?;
    }

    let mut exercised_signals = HashSet::new();
    let DispatchOutcome {
        frontier_branches,
        cursor,
        mut frontier_pruned_covered,
    } = dispatch_frontier(
        &mut queue,
        &mut seen_branches,
        &mut next_branch_ordinal,
        &mut exercised_signals,
        &mut store,
        max_actions,
    )?;

    let frontier_pending = collect_pending(
        &queue,
        cursor,
        &exercised_signals,
        &mut frontier_pruned_covered,
    );
    validate_captured_checkpoints(&job_rows, &frontier_branches, &frontier_pending, &store)?;
    let report = build_report(
        job_rows,
        frontier_branches,
        frontier_pending,
        frontier_pruned_covered,
    );

    Ok(FrontierRun {
        checkpoints: store.into_ordered(),
        report,
    })
}

fn run_jobs(
    jobs: &[FrontierJob],
    max_actions: u64,
    store: &mut CheckpointStore,
) -> Result<Vec<Value>, FrontierError> {
    let mut job_rows = Vec::with_capacity(jobs.len());
    for job in jobs {
        let session = Session::new(&job.manifest_json)?;
        create_game_checkpoint(&session)?;
        let result = absorb_rollout(store, novelty::run_novelty_rollout(&session, max_actions)?)?;
        job_rows.push(json!({
            "jobId": job.job_id,
            "lessonId": job.lesson_id,
            "orientation": job.orientation,
            "result": result,
        }));
    }
    Ok(job_rows)
}

struct DispatchOutcome {
    cursor: usize,
    frontier_branches: Vec<Value>,
    frontier_pruned_covered: u64,
}

fn dispatch_frontier(
    queue: &mut Vec<FrontierSeed>,
    seen_branches: &mut HashSet<String>,
    next_branch_ordinal: &mut u64,
    exercised_signals: &mut HashSet<String>,
    store: &mut CheckpointStore,
    max_actions: u64,
) -> Result<DispatchOutcome, FrontierError> {
    let mut frontier_branches = Vec::new();
    let mut cursor = 0usize;
    let mut frontier_pruned_covered = 0u64;
    let mut stop_after_failure = false;
    while cursor < queue.len()
        && frontier_branches.len() < FRONTIER_BRANCH_LIMIT
        && !stop_after_failure
    {
        let seed = queue[cursor].clone();
        cursor += 1;
        let novel = novel_signals(&seed.signals, exercised_signals);
        if novel.is_empty() {
            frontier_pruned_covered += 1;
            continue;
        }
        let (entry_event_types, session) = step_frontier_seed(&seed, store)?;
        mark_exercised(exercised_signals, &seed.action_kind, &entry_event_types);
        let result = absorb_rollout(store, novelty::run_novelty_rollout(&session, max_actions)?)?;
        if result["initialStateHash"].as_str() != Some(seed.predicted_state_hash.as_str()) {
            return Err(FrontierError::Invalid(
                "private novelty frontier rollout started from the wrong state".to_owned(),
            ));
        }
        let status = result["status"].as_str().unwrap_or_default().to_owned();
        let child_frontier = result["frontier"].clone();
        frontier_branches.push(branch_row(&seed, &novel, entry_event_types, result));
        if status == "failed" {
            stop_after_failure = true;
        } else {
            enqueue_frontier(
                queue,
                seen_branches,
                next_branch_ordinal,
                &child_frontier,
                &seed.parent_job_id,
                Some(seed.branch_id.as_str()),
                seed.depth + 1,
            )?;
        }
    }
    Ok(DispatchOutcome {
        cursor,
        frontier_branches,
        frontier_pruned_covered,
    })
}

fn step_frontier_seed(
    seed: &FrontierSeed,
    store: &CheckpointStore,
) -> Result<(Vec<String>, Session), FrontierError> {
    let checkpoint_value = store.get(&seed.checkpoint_id).ok_or_else(|| {
        FrontierError::Invalid("private novelty frontier checkpoint was not captured".to_owned())
    })?;
    let checkpoint = parse_game_checkpoint(&canonical_json(checkpoint_value)?)?;
    let mut session = resume_game_checkpoint(&checkpoint)?;
    let issued = session
        .legal_actions()?
        .into_iter()
        .filter(|action| action.action_id.as_str() == seed.action_id)
        .collect::<Vec<_>>();
    if issued.len() != 1 {
        return Err(FrontierError::Invalid(
            "private novelty frontier action is stale".to_owned(),
        ));
    }
    let action = &issued[0];
    let action_kind = action.descriptor["kind"].as_str().unwrap_or_default();
    if action_kind != seed.action_kind {
        return Err(FrontierError::Invalid(
            "private novelty frontier action kind changed".to_owned(),
        ));
    }
    let receipt = match session.step(request_for(action))? {
        StepResult::Accepted(receipt) => receipt,
        StepResult::Rejected(rejection) => {
            return Err(FrontierError::Invalid(format!(
                "private novelty frontier action rejected: {}",
                rejection_code_name(rejection.code)
            )));
        }
    };
    let entry_event_types = unique_sorted_event_types(&receipt.events);
    let entry_hash = hash_state(&session.replay_value()?["state"])?;
    if entry_hash != seed.predicted_state_hash
        || canonical_json(&seed.predicted_event_types)?
            != canonical_json(&json!(entry_event_types))?
        || seed.signals.iter().any(|signal| {
            if signal.kind == "action-kind" {
                signal.value != seed.action_kind
            } else {
                !entry_event_types.iter().any(|event| event == &signal.value)
            }
        })
    {
        return Err(FrontierError::Invalid(
            "private novelty frontier prediction did not replay exactly".to_owned(),
        ));
    }
    Ok((entry_event_types, session))
}

fn collect_pending(
    queue: &[FrontierSeed],
    cursor: usize,
    exercised_signals: &HashSet<String>,
    frontier_pruned_covered: &mut u64,
) -> Vec<Value> {
    let mut frontier_pending = Vec::new();
    for seed in queue.iter().skip(cursor) {
        let signals = novel_signals(&seed.signals, exercised_signals);
        if signals.is_empty() {
            *frontier_pruned_covered += 1;
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
            "signals": signals_json(&signals),
        }));
    }
    frontier_pending
}

fn validate_captured_checkpoints(
    job_rows: &[Value],
    frontier_branches: &[Value],
    frontier_pending: &[Value],
    store: &CheckpointStore,
) -> Result<(), FrontierError> {
    for result in job_rows
        .iter()
        .map(|row| &row["result"])
        .chain(frontier_branches.iter().map(|branch| &branch["result"]))
    {
        ensure_reported_checkpoints(result, store)?;
    }
    for pending in frontier_pending {
        let checkpoint_id = pending["checkpointId"].as_str().unwrap_or_default();
        if !store.contains(checkpoint_id) {
            return Err(FrontierError::Invalid(
                "private novelty gauntlet did not capture a pending checkpoint".to_owned(),
            ));
        }
    }
    Ok(())
}

fn build_report(
    job_rows: Vec<Value>,
    frontier_branches: Vec<Value>,
    frontier_pending: Vec<Value>,
    frontier_pruned_covered: u64,
) -> Value {
    let pending_signals = pending_signal_count(&frontier_pending);
    let frontier_limit_reached =
        frontier_branches.len() == FRONTIER_BRANCH_LIMIT && !frontier_pending.is_empty();
    let frontier_max_depth = frontier_branches
        .iter()
        .filter_map(|branch| branch["depth"].as_u64())
        .max()
        .unwrap_or(0);
    let totals = json!({
        "branchLimit": FRONTIER_BRANCH_LIMIT,
        "completed": status_count(&job_rows, "completed"),
        "failed": status_count(&job_rows, "failed"),
        "frontierBranches": frontier_branches.len(),
        "frontierCompleted": branch_status_count(&frontier_branches, "completed"),
        "frontierFailed": branch_status_count(&frontier_branches, "failed"),
        "frontierHorizon": branch_status_count(&frontier_branches, "horizon"),
        "frontierLimitReached": frontier_limit_reached,
        "frontierMaxDepth": frontier_max_depth,
        "frontierPending": frontier_pending.len(),
        "frontierPendingSignals": pending_signals,
        "frontierPrunedCovered": frontier_pruned_covered,
        "horizon": status_count(&job_rows, "horizon"),
        "jobs": 4,
        "savedCheckpoints": 0,
    });
    let mut report = Map::new();
    report.insert(
        "classification".to_owned(),
        Value::String("authority-private".to_owned()),
    );
    report.insert(
        "frontierBranches".to_owned(),
        Value::Array(frontier_branches),
    );
    report.insert("frontierPending".to_owned(), Value::Array(frontier_pending));
    report.insert("jobs".to_owned(), Value::Array(job_rows));
    report.insert(
        "policyVersion".to_owned(),
        Value::String(POLICY_VERSION.to_owned()),
    );
    report.insert("schemaVersion".to_owned(), json!(2));
    report.insert("totals".to_owned(), totals);
    Value::Object(report)
}

fn status_count(rows: &[Value], status: &str) -> usize {
    rows.iter()
        .filter(|row| row["result"]["status"] == status)
        .count()
}

fn branch_status_count(branches: &[Value], status: &str) -> usize {
    branches
        .iter()
        .filter(|branch| branch["result"]["status"] == status)
        .count()
}

fn pending_signal_count(frontier_pending: &[Value]) -> usize {
    let mut pending_signals = HashSet::new();
    for pending in frontier_pending {
        if let Some(signals) = pending["signals"].as_array() {
            for signal in signals {
                let kind = signal["kind"].as_str().unwrap_or_default();
                let value = signal["value"].as_str().unwrap_or_default();
                pending_signals.insert(format!("{kind}:\0{value}"));
            }
        }
    }
    pending_signals.len()
}

fn novel_signals(signals: &[Signal], exercised: &HashSet<String>) -> Vec<Signal> {
    signals
        .iter()
        .filter(|signal| !exercised.contains(&signal_key(signal)))
        .cloned()
        .collect()
}

fn mark_exercised(exercised: &mut HashSet<String>, action_kind: &str, event_types: &[String]) {
    exercised.insert(signal_key(&Signal {
        kind: "action-kind".to_owned(),
        value: action_kind.to_owned(),
    }));
    for event_type in event_types {
        exercised.insert(signal_key(&Signal {
            kind: "event-type".to_owned(),
            value: event_type.clone(),
        }));
    }
}

fn branch_row(
    seed: &FrontierSeed,
    novel: &[Signal],
    entry_event_types: Vec<String>,
    result: Value,
) -> Value {
    let mut row = Map::new();
    row.insert("actionId".to_owned(), json!(seed.action_id));
    row.insert("actionKind".to_owned(), json!(seed.action_kind));
    row.insert("branchId".to_owned(), json!(seed.branch_id));
    row.insert("checkpointId".to_owned(), json!(seed.checkpoint_id));
    row.insert("depth".to_owned(), json!(seed.depth));
    row.insert("entryActionCount".to_owned(), json!(1));
    row.insert(
        "entryEventTypes".to_owned(),
        Value::Array(entry_event_types.into_iter().map(Value::String).collect()),
    );
    row.insert(
        "novelSignalsAtDispatch".to_owned(),
        Value::Array(signals_json(novel)),
    );
    row.insert(
        "parentBranchId".to_owned(),
        match &seed.parent_branch_id {
            Some(id) => Value::String(id.clone()),
            None => Value::Null,
        },
    );
    row.insert("parentJobId".to_owned(), json!(seed.parent_job_id));
    row.insert("result".to_owned(), result);
    row.insert(
        "signals".to_owned(),
        Value::Array(signals_json(&seed.signals)),
    );
    Value::Object(row)
}

fn absorb_rollout(
    store: &mut CheckpointStore,
    rollout: novelty::NoveltyRollout,
) -> Result<Value, FrontierError> {
    for emission in &rollout.emissions {
        match store.capture(&emission.checkpoint) {
            Ok(()) => {}
            Err(CaptureOutcome::Collision) => {
                return Ok(emission.failure_if_sink_rejects.clone());
            }
            Err(CaptureOutcome::Error(error)) => return Err(error),
        }
    }
    Ok(rollout.result)
}

fn enqueue_frontier(
    queue: &mut Vec<FrontierSeed>,
    seen_branches: &mut HashSet<String>,
    next_branch_ordinal: &mut u64,
    frontier: &Value,
    parent_job_id: &str,
    parent_branch_id: Option<&str>,
    depth: u64,
) -> Result<(), FrontierError> {
    let Some(candidates) = frontier.as_array() else {
        return Ok(());
    };
    let mut grouped: HashMap<String, FrontierSeed> = HashMap::new();
    for candidate in candidates {
        let action_id = required_str(candidate, "actionId")?;
        let action_kind = required_str(candidate, "actionKind")?;
        let checkpoint_id = required_str(candidate, "checkpointId")?;
        let predicted_state_hash = required_str(candidate, "predictedStateHash")?;
        let predicted_event_types = candidate
            .get("predictedEventTypes")
            .cloned()
            .unwrap_or(Value::Null);
        let signal = Signal {
            kind: required_str(&candidate["signal"], "kind")?,
            value: required_str(&candidate["signal"], "value")?,
        };
        let key = format!("{checkpoint_id}\0{action_id}");
        if let Some(existing) = grouped.get_mut(&key) {
            if existing.action_kind != action_kind
                || existing.predicted_state_hash != predicted_state_hash
                || canonical_json(&existing.predicted_event_types)?
                    != canonical_json(&predicted_event_types)?
            {
                return Err(FrontierError::Invalid(
                    "private novelty frontier action has inconsistent predictions".to_owned(),
                ));
            }
            existing.signals.push(signal);
        } else {
            grouped.insert(
                key,
                FrontierSeed {
                    action_id,
                    action_kind,
                    branch_id: String::new(),
                    checkpoint_id,
                    depth,
                    parent_branch_id: parent_branch_id.map(str::to_owned),
                    parent_job_id: parent_job_id.to_owned(),
                    predicted_event_types,
                    predicted_state_hash,
                    signals: vec![signal],
                },
            );
        }
    }

    let mut seeds = grouped.into_values().collect::<Vec<_>>();
    seeds.sort_by(|left, right| {
        left.checkpoint_id
            .cmp(&right.checkpoint_id)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    for mut seed in seeds {
        let key = format!("{}\0{}", seed.checkpoint_id, seed.action_id);
        if !seen_branches.insert(key) {
            continue;
        }
        seed.signals.sort_by_key(signal_key);
        seed.branch_id = format!("{parent_job_id}:branch-{next_branch_ordinal}");
        *next_branch_ordinal += 1;
        queue.push(seed);
    }
    Ok(())
}

fn ensure_reported_checkpoints(
    result: &Value,
    store: &CheckpointStore,
) -> Result<(), FrontierError> {
    if let Some(frontier) = result["frontier"].as_array() {
        for candidate in frontier {
            let checkpoint_id = candidate["checkpointId"].as_str().unwrap_or_default();
            if !store.contains(checkpoint_id) {
                return Err(FrontierError::Invalid(
                    "private novelty gauntlet did not capture a reported checkpoint".to_owned(),
                ));
            }
        }
    }
    if result["status"] == "horizon" {
        let checkpoint_id = result["checkpointId"].as_str().unwrap_or_default();
        if !store.contains(checkpoint_id) {
            return Err(FrontierError::Invalid(
                "private novelty gauntlet did not capture a reported checkpoint".to_owned(),
            ));
        }
    }
    if result["status"] == "failed"
        && let Some(checkpoint_id) = result["failure"]
            .get("checkpointId")
            .and_then(Value::as_str)
        && !store.contains(checkpoint_id)
    {
        return Err(FrontierError::Invalid(
            "private novelty gauntlet did not capture a reported checkpoint".to_owned(),
        ));
    }
    Ok(())
}

fn checkpoint_bytes(checkpoint: &Value) -> Result<String, FrontierError> {
    let parsed = parse_game_checkpoint(&canonical_json(checkpoint)?)?;
    Ok(serialize_game_checkpoint(&parsed)?)
}

fn signals_json(signals: &[Signal]) -> Vec<Value> {
    signals
        .iter()
        .map(|signal| {
            json!({
                "kind": signal.kind,
                "value": signal.value,
            })
        })
        .collect()
}

fn signal_key(signal: &Signal) -> String {
    format!("{}:\0{}", signal.kind, signal.value)
}

fn required_str(value: &Value, field: &str) -> Result<String, FrontierError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| FrontierError::Invalid(format!("private novelty frontier missing {field}")))
}

fn unique_sorted_event_types(events: &[crate::contract::Event]) -> Vec<String> {
    let mut types = events
        .iter()
        .map(|event| event.event_type.clone())
        .collect::<Vec<_>>();
    types.sort();
    types.dedup();
    types
}

fn rejection_code_name(code: RejectionCode) -> &'static str {
    match code {
        RejectionCode::StaleVersion => "stale_version",
        RejectionCode::TerminalState => "terminal_state",
        RejectionCode::UnknownAction => "unknown_action",
        RejectionCode::WrongSeat => "wrong_seat",
    }
}

/// Builds the CLI response object `{ report, checkpoints }`.
#[must_use]
pub fn frontier_response_value(run: &FrontierRun) -> Value {
    let checkpoints = run
        .checkpoints
        .iter()
        .map(|(checkpoint_id, checkpoint)| {
            let mut row = Map::new();
            row.insert("checkpoint".to_owned(), checkpoint.clone());
            row.insert("checkpointId".to_owned(), json!(checkpoint_id));
            Value::Object(row)
        })
        .collect::<Vec<_>>();
    json!({
        "checkpoints": checkpoints,
        "report": run.report,
    })
}
