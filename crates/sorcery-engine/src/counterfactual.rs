//! Checkpoint-rooted counterfactual search over engine-issued legal actions.

use serde_json::{Value, json};

use crate::contract::{ActionRequest, LegalAction, Seat};
use crate::session::{Session, SessionError, StepResult};

/// Widest root-action list that still expands every branch.
pub const COUNTERFACTUAL_ROOT_LIMIT: usize = 128;
/// Longest policy continuation after the forced root action.
pub const COUNTERFACTUAL_CONTINUATION_LIMIT: usize = 32;

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
    if max_continuation_decisions > COUNTERFACTUAL_CONTINUATION_LIMIT {
        return Err(SessionError::Novelty {
            message: format!(
                "maxContinuationDecisions must be 0-{COUNTERFACTUAL_CONTINUATION_LIMIT}"
            ),
        });
    }
    let perspective = root.decision_seat();
    let root_state_hash = root.state_hash()?;
    let actions = root.legal_actions()?;
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

    let mut branches = Vec::with_capacity(actions.len());
    let mut terminals = Vec::new();
    for action in &actions {
        let branch = rollout_branch(root, action, max_continuation_decisions, perspective)?;
        if branch["outcome"] == "terminal" {
            terminals.push(branch.clone());
        }
        branches.push(branch);
    }
    result["branches"] = Value::Array(branches);
    result["recommendation"] = if terminals.len() == actions.len() {
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
    Ok(CounterfactualReport { result })
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
    use super::{COUNTERFACTUAL_CONTINUATION_LIMIT, run_counterfactual};
    use crate::session::Session;
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

    fn json_hash(session: &Session) -> serde_json::Value {
        serde_json::json!(session.state_hash().expect("hash"))
    }
}
