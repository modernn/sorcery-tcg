//! Checkpoint-rooted counterfactual search over engine-issued legal actions.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

use crate::contract::{ActionRequest, LegalAction, Seat};
use crate::session::{Session, SessionError, StepResult};

/// Widest root-action list that still expands every branch.
pub const COUNTERFACTUAL_ROOT_LIMIT: usize = 128;
/// Longest policy continuation after the forced root action.
pub const COUNTERFACTUAL_CONTINUATION_LIMIT: usize = 32;
/// Maximum number of speculative workers owned by one counterfactual search.
pub const COUNTERFACTUAL_WORKER_LIMIT: usize = 8;
/// Conservative multiplier for clone capacities and transient serialization data.
pub const COUNTERFACTUAL_CLONE_ALLOWANCE_FACTOR: usize = 2;
/// Maximum estimated bytes retained by one parallel counterfactual search.
pub const COUNTERFACTUAL_PARALLEL_BYTES_LIMIT: usize = 64 * 1024 * 1024;

type IndexedResult<T> = (usize, Result<T, SessionError>);

thread_local! {
    static OUTER_WORKER_OWNS_PARALLELISM: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct OuterWorkerGuard {
    previous: bool,
}

impl Drop for OuterWorkerGuard {
    fn drop(&mut self) {
        OUTER_WORKER_OWNS_PARALLELISM.with(|owned| owned.set(self.previous));
    }
}

pub(crate) fn enter_outer_worker() -> OuterWorkerGuard {
    let previous = OUTER_WORKER_OWNS_PARALLELISM.with(|owned| owned.replace(true));
    OuterWorkerGuard { previous }
}

/// Public counterfactual report from one authoritative snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterfactualReport {
    result: Value,
}

impl CounterfactualReport {
    /// Returns the public report object.
    #[must_use]
    pub const fn result(&self) -> &Value {
        &self.result
    }
}

/// Expands every engine-issued root action, then follows the baseline policy.
///
/// The caller's session is not mutated. Horizons are not scored. A recommendation
/// is emitted only when every branch finishes.
///
/// # Errors
///
/// Returns [`SessionError`] when the continuation bound is outside `0..=32` or a
/// forced or policy action cannot be applied.
pub fn run_counterfactual(
    root: &Session,
    max_continuation_decisions: usize,
) -> Result<CounterfactualReport, SessionError> {
    run_counterfactual_with_workers(root, max_continuation_decisions, 1)
}

/// Expands counterfactual root actions with a bounded search-owned worker budget.
///
/// The default [`run_counterfactual`] path remains serial. This option owns only
/// workers inside this search; callers already running an outer worker pool must
/// pass `workers: 1` so the outer pool remains the sole owner of parallelism.
/// If the conservative parallel-memory estimate is over budget, the search falls
/// back to this same serial path and returns the identical report.
///
/// # Errors
///
/// Returns [`SessionError`] when a bound, worker budget, nested ownership check,
/// forced action, or deterministic continuation fails. An over-budget memory
/// estimate falls back to serial execution rather than failing the search.
pub fn run_counterfactual_with_workers(
    root: &Session,
    max_continuation_decisions: usize,
    workers: usize,
) -> Result<CounterfactualReport, SessionError> {
    if max_continuation_decisions > COUNTERFACTUAL_CONTINUATION_LIMIT {
        return Err(SessionError::Novelty {
            message: format!(
                "maxContinuationDecisions must be 0-{COUNTERFACTUAL_CONTINUATION_LIMIT}"
            ),
        });
    }
    if !(1..=COUNTERFACTUAL_WORKER_LIMIT).contains(&workers) {
        return Err(SessionError::Novelty {
            message: format!("counterfactual workers must be 1-{COUNTERFACTUAL_WORKER_LIMIT}"),
        });
    }
    let perspective = root.decision_seat();
    let root_state_hash = root.state_hash()?;
    let actions = root.legal_actions()?;
    let effective_workers = workers.min(actions.len());
    if effective_workers > 1 && OUTER_WORKER_OWNS_PARALLELISM.with(Cell::get) {
        return Err(SessionError::Novelty {
            message: "outer worker owns parallelism; counterfactual workers must be 1".to_owned(),
        });
    }
    let mut result = json!({
        "classification": "authority-private-counterfactual",
        "maxContinuationDecisions": max_continuation_decisions,
        "policyVersion": "deterministic-demo-v1",
        "rootActionCount": actions.len(),
        "rootActionLimit": COUNTERFACTUAL_ROOT_LIMIT,
        "rootStateHash": root_state_hash,
    });
    if actions.len() > COUNTERFACTUAL_ROOT_LIMIT {
        result["branches"] = json!([]);
        result["recommendation"] = Value::Null;
        result["status"] = json!("too-wide");
        return Ok(CounterfactualReport { result });
    }

    if effective_workers <= 1 {
        return run_serial_counterfactual(
            result,
            root,
            &actions,
            max_continuation_decisions,
            perspective,
        );
    }

    let root_bytes = root.serialized_clone_size()?;
    let Some(branch_steps) = max_continuation_decisions.checked_add(1) else {
        return run_serial_counterfactual(
            result,
            root,
            &actions,
            max_continuation_decisions,
            perspective,
        );
    };
    let Some(estimated_parallel_bytes) = root_bytes
        .checked_mul(COUNTERFACTUAL_CLONE_ALLOWANCE_FACTOR)
        .and_then(|bytes| bytes.checked_mul(branch_steps))
        .and_then(|bytes| bytes.checked_mul(effective_workers))
    else {
        return run_serial_counterfactual(
            result,
            root,
            &actions,
            max_continuation_decisions,
            perspective,
        );
    };
    if estimated_parallel_bytes > COUNTERFACTUAL_PARALLEL_BYTES_LIMIT {
        return run_serial_counterfactual(
            result,
            root,
            &actions,
            max_continuation_decisions,
            perspective,
        );
    }
    let indexed_branches = parallel_indexed(actions.len(), effective_workers, |action_index| {
        rollout_branch(
            root,
            &actions[action_index],
            max_continuation_decisions,
            perspective,
        )
    })?;
    let branches = indexed_branches
        .into_iter()
        .map(|(_, branch)| branch)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(finish_counterfactual(result, actions.len(), branches))
}

fn run_serial_counterfactual(
    result: Value,
    root: &Session,
    actions: &[LegalAction],
    max_continuation_decisions: usize,
    perspective: Seat,
) -> Result<CounterfactualReport, SessionError> {
    let mut branches = Vec::with_capacity(actions.len());
    for action in actions {
        branches.push(rollout_branch(
            root,
            action,
            max_continuation_decisions,
            perspective,
        )?);
    }
    Ok(finish_counterfactual(result, actions.len(), branches))
}

fn finish_counterfactual(
    mut result: Value,
    action_count: usize,
    branches: Vec<Value>,
) -> CounterfactualReport {
    let terminals = branches
        .iter()
        .filter(|branch| branch["outcome"] == "terminal")
        .cloned()
        .collect::<Vec<_>>();
    result["branches"] = Value::Array(branches);
    result["recommendation"] = if terminals.len() == action_count {
        if let Some(preferred) = terminals.into_iter().reduce(|left, right| {
            if preferred_terminal(&left, &right) {
                left
            } else {
                right
            }
        }) {
            json!({
                "rootActionId": preferred["rootActionId"],
                "score": preferred["score"],
            })
        } else {
            Value::Null
        }
    } else {
        Value::Null
    };
    result["status"] = json!("complete");
    CounterfactualReport { result }
}

fn parallel_indexed<T, F>(
    item_count: usize,
    workers: usize,
    work: F,
) -> Result<Vec<IndexedResult<T>>, SessionError>
where
    T: Send,
    F: Fn(usize) -> Result<T, SessionError> + Send + Sync,
{
    let worker_count = workers.min(item_count);
    let next_item = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles = (0..worker_count)
            .map(|_| {
                scope.spawn(|| {
                    let mut results = Vec::new();
                    loop {
                        let index = next_item.fetch_add(1, Ordering::Relaxed);
                        if index >= item_count {
                            break;
                        }
                        results.push((index, work(index)));
                    }
                    results
                })
            })
            .collect::<Vec<_>>();
        let mut results = Vec::with_capacity(item_count);
        for handle in handles {
            results.extend(handle.join().map_err(|_| SessionError::Novelty {
                message: "counterfactual worker panicked".to_owned(),
            })?);
        }
        results.sort_unstable_by_key(|(index, _)| *index);
        Ok(results)
    })
}

fn rollout_branch(
    root: &Session,
    action: &LegalAction,
    max_continuation_decisions: usize,
    perspective: Seat,
) -> Result<Value, SessionError> {
    let mut live = root.clone();
    match live.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    })? {
        StepResult::Accepted(_) => {}
        StepResult::Rejected(_) => {
            return Err(SessionError::Novelty {
                message: "engine rejected issued root action".to_owned(),
            });
        }
    }
    let mut decision_count: u64 = 1;
    let max_continuation =
        u64::try_from(max_continuation_decisions).map_err(|_| SessionError::SequenceExhausted)?;
    while live.outcome().is_none() && decision_count <= max_continuation {
        let selected = live.select_baseline_policy_action()?;
        match live.step(ActionRequest {
            action_id: selected.action_id.to_string(),
            seat: selected.seat,
            state_version: selected.state_version,
        })? {
            StepResult::Accepted(_) => {}
            StepResult::Rejected(_) => {
                return Err(SessionError::Novelty {
                    message: "engine rejected issued continuation".to_owned(),
                });
            }
        }
        decision_count = decision_count
            .checked_add(1)
            .ok_or(SessionError::SequenceExhausted)?;
    }
    let common = json!({
        "decisionCount": decision_count,
        "finalStateHash": live.state_hash()?,
        "rootActionId": action.action_id,
    });
    if live.outcome().is_some() {
        let terminal = live.replay_value()?["state"]["terminal"].clone();
        let mut branch = common;
        branch["outcome"] = json!("terminal");
        branch["score"] = json!(terminal_score(&terminal, perspective));
        branch["terminal"] = terminal;
        Ok(branch)
    } else {
        let mut branch = common;
        branch["outcome"] = json!("unknown");
        branch["reason"] = json!("horizon");
        Ok(branch)
    }
}

fn terminal_score(terminal: &Value, perspective: Seat) -> i8 {
    if terminal.get("result").is_some() {
        return 0;
    }
    let winner = terminal.get("winner").and_then(Value::as_str);
    let perspective = match perspective {
        Seat::North => "north",
        Seat::South => "south",
    };
    if winner == Some(perspective) { 1 } else { -1 }
}

fn preferred_terminal(left: &Value, right: &Value) -> bool {
    let left_score = left["score"].as_i64().unwrap_or(0);
    let right_score = right["score"].as_i64().unwrap_or(0);
    if left_score != right_score {
        return left_score > right_score;
    }
    let left_decisions = left["decisionCount"].as_u64().unwrap_or(0);
    let right_decisions = right["decisionCount"].as_u64().unwrap_or(0);
    if left_score == 1 && left_decisions != right_decisions {
        return left_decisions < right_decisions;
    }
    if left_score == -1 && left_decisions != right_decisions {
        return left_decisions > right_decisions;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        COUNTERFACTUAL_CLONE_ALLOWANCE_FACTOR, COUNTERFACTUAL_CONTINUATION_LIMIT,
        COUNTERFACTUAL_PARALLEL_BYTES_LIMIT, COUNTERFACTUAL_WORKER_LIMIT, run_counterfactual,
        run_counterfactual_with_workers,
    };
    use crate::contract::ActionRequest;
    use crate::session::Session;
    use crate::session::StepResult;
    use crate::synthetic::synthetic_demo_session;

    #[test]
    fn opening_counterfactual_is_too_wide_or_complete() {
        let session = synthetic_demo_session(31);
        let report = run_counterfactual(&session, 0).expect("counterfactual");
        assert!(report.result()["status"] == "complete" || report.result()["status"] == "too-wide");
        assert_eq!(report.result()["rootStateHash"], json_hash(&session));
    }

    #[test]
    fn rejects_an_oversize_continuation() {
        let session = synthetic_demo_session(31);
        let error = run_counterfactual(&session, COUNTERFACTUAL_CONTINUATION_LIMIT + 1)
            .expect_err("oversize");
        assert!(error.to_string().contains("maxContinuationDecisions"));
    }

    #[test]
    fn worker_counts_preserve_complete_report_and_root_identity() {
        let session = synthetic_demo_session(31);
        let root_hash = session.state_hash().expect("root hash");
        let serial = run_counterfactual(&session, 4).expect("serial counterfactual");
        for workers in [2, 4, COUNTERFACTUAL_WORKER_LIMIT] {
            let parallel = run_counterfactual_with_workers(&session, 4, workers)
                .expect("parallel counterfactual");
            assert_eq!(serial.result(), parallel.result());
        }
        assert_eq!(root_hash, session.state_hash().expect("root hash"));
    }

    #[test]
    fn rejects_an_invalid_worker_budget() {
        let session = synthetic_demo_session(31);
        for workers in [0, COUNTERFACTUAL_WORKER_LIMIT + 1] {
            let error =
                run_counterfactual_with_workers(&session, 0, workers).expect_err("invalid workers");
            assert!(error.to_string().contains("counterfactual workers"));
        }
    }

    #[test]
    fn outer_worker_guard_rejects_nested_parallelism() {
        let session = synthetic_demo_session(31);
        let _guard = super::enter_outer_worker();
        let error = run_counterfactual_with_workers(&session, 0, 2)
            .expect_err("nested counterfactual workers");
        assert!(error.to_string().contains("outer worker owns parallelism"));
    }

    #[test]
    fn large_rejected_attempt_journal_falls_back_to_serial() {
        let mut session = synthetic_demo_session(31);
        let oversized_action_id = "rejected-".to_owned() + &"x".repeat(8 * 1024);
        for _ in 0..600 {
            let result = session
                .step(ActionRequest {
                    action_id: oversized_action_id.clone(),
                    seat: session.acting_controller(),
                    state_version: session.state_version().saturating_add(1),
                })
                .expect("rejected request");
            assert!(matches!(result, StepResult::Rejected(_)));
        }
        let serialized_bytes = session.serialized_clone_size().expect("serialized size");
        let minimum_rejection_size =
            COUNTERFACTUAL_PARALLEL_BYTES_LIMIT / (COUNTERFACTUAL_CLONE_ALLOWANCE_FACTOR * 5 * 2);
        assert!(serialized_bytes > minimum_rejection_size);
        let serial = run_counterfactual(&session, 4).expect("serial fallback baseline");
        let fallback = run_counterfactual_with_workers(&session, 4, 2)
            .expect("large rejected-attempt journal should fall back to serial");
        assert_eq!(serial.result(), fallback.result());
        assert_eq!(
            serial.result()["rootStateHash"],
            fallback.result()["rootStateHash"]
        );
    }

    #[test]
    fn one_action_root_uses_serial_admission() {
        let mut session = synthetic_demo_session(31);
        for _ in 0..128 {
            if session.legal_actions().expect("legal actions").len() == 1 {
                let report =
                    run_counterfactual_with_workers(&session, 0, COUNTERFACTUAL_WORKER_LIMIT)
                        .expect("one-action root should not be falsely rejected");
                assert_eq!(report.result()["status"], "complete");
                return;
            }
            if session.outcome().is_some() {
                break;
            }
            let action = session
                .select_baseline_policy_action()
                .expect("baseline action");
            assert!(matches!(
                session
                    .step(ActionRequest {
                        action_id: action.action_id.to_string(),
                        seat: action.seat,
                        state_version: action.state_version,
                    })
                    .expect("baseline step"),
                StepResult::Accepted(_)
            ));
        }
        panic!("synthetic session did not reach a one-action root");
    }

    fn json_hash(session: &Session) -> serde_json::Value {
        serde_json::json!(session.state_hash().expect("hash"))
    }
}
