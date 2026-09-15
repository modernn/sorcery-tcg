//! Authority-private counterfactual root-action search.

use std::fmt;

use serde_json::{Value, json};

use crate::canonical::{CanonicalError, identity_hash};
use crate::contract::{LegalAction, Seat};
use crate::novelty::{request_for, select_index};
use crate::session::{Session, SessionError, StepResult};

/// Maximum root actions before the search reports `too-wide`.
pub const MAX_ROOT_ACTIONS: usize = 128;
/// Maximum continuation decisions after the root choice.
pub const MAX_CONTINUATION_ACTIONS: u64 = 32;
const POLICY_VERSION: &str = "deterministic-demo-v1";

/// Counterfactual search report matching the previous TypeScript contract.
#[derive(Clone, Debug)]
pub struct CounterfactualReport {
    /// Canonical JSON report body.
    pub result: Value,
}

/// Counterfactual search failed.
#[derive(Debug)]
pub enum CounterfactualError {
    /// Continuation bound was outside `0..=32`.
    InvalidLimit,
    /// Session transition failed.
    Session(SessionError),
    /// Canonical hashing failed.
    Canonical(CanonicalError),
    /// JSON encoding failed.
    Json(serde_json::Error),
    /// Deterministic selector could not choose from issued actions.
    Selector,
    /// Engine rejected an issued action.
    Rejected(String),
}

impl From<SessionError> for CounterfactualError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<CanonicalError> for CounterfactualError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for CounterfactualError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl fmt::Display for CounterfactualError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => {
                formatter.write_str("counterfactual maxContinuationDecisions must be 0-32")
            }
            Self::Session(error) => write!(formatter, "{error}"),
            Self::Canonical(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Selector => formatter.write_str("deterministic selector failed"),
            Self::Rejected(code) => write!(formatter, "engine rejected issued action: {code}"),
        }
    }
}

/// Runs one counterfactual search from the live session.
///
/// # Errors
///
/// Returns [`CounterfactualError`] for invalid limits or engine failures.
pub fn run_counterfactual_rollouts(
    session: &Session,
    max_continuation_decisions: u64,
) -> Result<CounterfactualReport, CounterfactualError> {
    if max_continuation_decisions > MAX_CONTINUATION_ACTIONS {
        return Err(CounterfactualError::InvalidLimit);
    }
    let actions = session.legal_actions()?;
    let replay = session.replay_value()?;
    let perspective = match replay["state"]["decisionSeat"].as_str() {
        Some("south") => Seat::South,
        _ => Seat::North,
    };
    let root_state_hash = session.state_hash()?.to_string();
    let base = json!({
        "classification": "authority-private-counterfactual",
        "maxContinuationDecisions": max_continuation_decisions,
        "policyVersion": POLICY_VERSION,
        "rootActionCount": actions.len(),
        "rootActionLimit": MAX_ROOT_ACTIONS,
        "rootStateHash": root_state_hash,
    });
    if actions.len() > MAX_ROOT_ACTIONS {
        return Ok(CounterfactualReport {
            result: merge(
                base,
                &json!({
                    "branches": [],
                    "recommendation": Value::Null,
                    "status": "too-wide",
                }),
            ),
        });
    }
    let mut branches = Vec::with_capacity(actions.len());
    for action in &actions {
        branches.push(rollout_branch(
            session,
            action,
            max_continuation_decisions,
            perspective,
        )?);
    }
    let terminal: Vec<_> = branches
        .iter()
        .filter(|branch| branch["outcome"] == "terminal")
        .cloned()
        .collect();
    let recommendation = if !branches.is_empty() && terminal.len() == branches.len() {
        Some(preferred_branch(&terminal))
    } else {
        None
    };
    Ok(CounterfactualReport {
        result: merge(
            base,
            &json!({
                "branches": branches,
                "recommendation": recommendation.map(|branch| json!({
                    "rootActionId": branch["rootActionId"].clone(),
                    "score": branch["score"].clone(),
                })),
                "status": "complete",
            }),
        ),
    })
}

fn rollout_branch(
    root: &Session,
    root_action: &LegalAction,
    max_continuation_decisions: u64,
    perspective: Seat,
) -> Result<Value, CounterfactualError> {
    let mut session = root.clone();
    match session.step(request_for(root_action))? {
        StepResult::Accepted(_) => {}
        StepResult::Rejected(rejection) => {
            return Err(CounterfactualError::Rejected(
                serde_json::to_value(rejection.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown_action".to_owned()),
            ));
        }
    }
    let mut decision_count = 1_u64;
    while session.replay_value()?["state"]["terminal"]["status"] == "active"
        && decision_count <= max_continuation_decisions
    {
        let actions = session.legal_actions()?;
        let replay = session.replay_value()?;
        let Some(index) = select_index(&replay, &actions) else {
            return Err(CounterfactualError::Selector);
        };
        match session.step(request_for(&actions[index]))? {
            StepResult::Accepted(_) => {}
            StepResult::Rejected(rejection) => {
                return Err(CounterfactualError::Rejected(
                    serde_json::to_value(rejection.code)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .unwrap_or_else(|| "unknown_action".to_owned()),
                ));
            }
        }
        decision_count += 1;
    }
    let replay = session.replay_value()?;
    let final_state_hash = identity_hash(&replay["state"])?.to_string();
    let common = json!({
        "decisionCount": decision_count,
        "finalStateHash": final_state_hash,
        "rootActionId": root_action.action_id.to_string(),
    });
    if replay["state"]["terminal"]["status"] == "finished" {
        let terminal = replay["state"]["terminal"].clone();
        let score = terminal_score(&terminal, perspective);
        Ok(merge(
            common,
            &json!({
                "outcome": "terminal",
                "score": score,
                "terminal": terminal,
            }),
        ))
    } else {
        Ok(merge(
            common,
            &json!({
                "outcome": "unknown",
                "reason": "horizon",
            }),
        ))
    }
}

fn terminal_score(terminal: &Value, perspective: Seat) -> i8 {
    if terminal.get("result").is_some() {
        return 0;
    }
    let winner = terminal["winner"].as_str().unwrap_or_default();
    let perspective = match perspective {
        Seat::North => "north",
        Seat::South => "south",
    };
    if winner == perspective { 1 } else { -1 }
}

fn preferred_branch(branches: &[Value]) -> Value {
    branches
        .iter()
        .cloned()
        .reduce(|left, right| {
            let left_score = left["score"].as_i64().unwrap_or(0);
            let right_score = right["score"].as_i64().unwrap_or(0);
            if left_score != right_score {
                return if left_score > right_score {
                    left
                } else {
                    right
                };
            }
            let left_decisions = left["decisionCount"].as_u64().unwrap_or(0);
            let right_decisions = right["decisionCount"].as_u64().unwrap_or(0);
            if left_score == 1 && left_decisions != right_decisions {
                return if left_decisions < right_decisions {
                    left
                } else {
                    right
                };
            }
            if left_score == -1 && left_decisions != right_decisions {
                return if left_decisions > right_decisions {
                    left
                } else {
                    right
                };
            }
            left
        })
        .unwrap_or(Value::Null)
}

fn merge(mut base: Value, extra: &Value) -> Value {
    if let (Some(object), Some(extra)) = (base.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            object.insert(key.clone(), value.clone());
        }
    }
    base
}
