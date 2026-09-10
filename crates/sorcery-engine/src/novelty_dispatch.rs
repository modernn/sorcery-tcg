//! Forced engine-issued action plus novelty rollout from the resulting snapshot.

use serde_json::{Value, json};

use crate::canonical::IdentityHash;
use crate::checkpoint::GameCheckpoint;
use crate::contract::{ActionRequest, RejectionCode};
use crate::novelty_rollout::{NoveltyRolloutOutput, run_novelty_rollout};
use crate::session::{Session, SessionError, StepResult};

/// One forced action plus the novelty report that starts after it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForcedNoveltyOutput {
    entry: ForcedNoveltyEntry,
    rollout: NoveltyRolloutOutput,
}

/// Public prediction-check for one forced engine-issued action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForcedNoveltyEntry {
    action_id: IdentityHash,
    action_kind: String,
    event_types: Vec<String>,
    state_hash: IdentityHash,
}

/// Inputs that identify one engine-issued action and its probe prediction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForcedNoveltyInput<'a> {
    /// Engine-issued action identity to force.
    pub action_id: &'a str,
    /// Descriptor kind that must still be bound to that action.
    pub action_kind: &'a str,
    /// Novelty horizon after the forced action.
    pub max_actions: usize,
    /// Sorted unique event types predicted by the one-step probe.
    pub predicted_event_types: &'a [String],
    /// Authoritative state hash predicted by the one-step probe.
    pub predicted_state_hash: &'a str,
}

impl ForcedNoveltyOutput {
    /// Returns the verified post-action entry.
    #[must_use]
    pub const fn entry(&self) -> &ForcedNoveltyEntry {
        &self.entry
    }

    /// Returns checkpoints emitted by the follow-up novelty rollout.
    #[must_use]
    pub fn emitted_checkpoints(&self) -> &[GameCheckpoint] {
        self.rollout.emitted_checkpoints()
    }

    /// Returns the public novelty report.
    #[must_use]
    pub const fn result(&self) -> &Value {
        self.rollout.result()
    }
}

impl ForcedNoveltyEntry {
    /// Returns the forced action identity.
    #[must_use]
    pub const fn action_id(&self) -> &IdentityHash {
        &self.action_id
    }

    /// Returns the forced descriptor kind.
    #[must_use]
    pub fn action_kind(&self) -> &str {
        &self.action_kind
    }

    /// Returns sorted unique event types from the accepted receipt.
    #[must_use]
    pub fn event_types(&self) -> &[String] {
        &self.event_types
    }

    /// Returns the authoritative hash after the forced action.
    #[must_use]
    pub const fn state_hash(&self) -> &IdentityHash {
        &self.state_hash
    }

    /// Materializes the public entry object.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "actionId": self.action_id,
            "actionKind": self.action_kind,
            "eventTypes": self.event_types,
            "stateHash": self.state_hash,
        })
    }
}

/// Forces one engine-issued action, checks its probe prediction, then rolls out novelty.
///
/// The caller's session is left at the post-action snapshot. The follow-up novelty
/// search clones from that snapshot and does not mutate it further.
///
/// # Errors
///
/// Returns [`SessionError`] when the action is stale, the prediction does not replay,
/// or novelty materialization fails.
pub fn run_novelty_from_forced_action(
    session: &mut Session,
    input: ForcedNoveltyInput<'_>,
) -> Result<ForcedNoveltyOutput, SessionError> {
    let mut issued = session
        .legal_actions()?
        .into_iter()
        .filter(|action| action.action_id.as_str() == input.action_id);
    let Some(action) = issued.next() else {
        return Err(dispatch_error("forced novelty action is stale"));
    };
    if issued.next().is_some() {
        return Err(dispatch_error("forced novelty action is stale"));
    }
    let actual_kind = match action.descriptor.get("kind") {
        Some(Value::String(kind)) => kind.clone(),
        _ => return Err(dispatch_error("forced novelty action omitted kind")),
    };
    if actual_kind != input.action_kind {
        return Err(dispatch_error("forced novelty action kind changed"));
    }
    let request = ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    };
    let receipt = match session.step(request)? {
        StepResult::Accepted(receipt) => receipt,
        StepResult::Rejected(rejection) => {
            return Err(dispatch_error(&format!(
                "forced novelty action rejected: {}",
                rejection_code_name(rejection.code)
            )));
        }
    };
    let mut event_types = receipt
        .events
        .iter()
        .map(|event| event.event_type.clone())
        .collect::<Vec<_>>();
    event_types.sort();
    event_types.dedup();
    let state_hash = session.state_hash()?;
    let predicted_hash = IdentityHash::parse(input.predicted_state_hash)
        .map_err(|_| dispatch_error("forced novelty predictedStateHash is invalid"))?;
    if state_hash != predicted_hash || event_types != input.predicted_event_types {
        return Err(dispatch_error(
            "forced novelty prediction did not replay exactly",
        ));
    }
    let rollout = run_novelty_rollout(session, input.max_actions)?;
    if rollout.result().get("initialStateHash") != Some(&json!(predicted_hash)) {
        return Err(dispatch_error(
            "forced novelty rollout started from the wrong state",
        ));
    }
    Ok(ForcedNoveltyOutput {
        entry: ForcedNoveltyEntry {
            action_id: action.action_id,
            action_kind: actual_kind,
            event_types,
            state_hash,
        },
        rollout,
    })
}

fn rejection_code_name(code: RejectionCode) -> &'static str {
    match code {
        RejectionCode::StaleVersion => "stale_version",
        RejectionCode::TerminalState => "terminal_state",
        RejectionCode::UnknownAction => "unknown_action",
        RejectionCode::WrongSeat => "wrong_seat",
    }
}

fn dispatch_error(message: &str) -> SessionError {
    SessionError::Novelty {
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ForcedNoveltyInput, run_novelty_from_forced_action};
    use crate::novelty_rollout::NOVELTY_ROLLOUT_ACTION_LIMIT;
    use crate::session::Session;
    use crate::synthetic::synthetic_demo_manifest_json;

    fn opening_session() -> Session {
        Session::new(&synthetic_demo_manifest_json(31).expect("manifest")).expect("session")
    }

    #[test]
    fn forces_the_opening_probe_and_starts_novelty_from_the_prediction() {
        let mut session = opening_session();
        let step = session.probe_novelty(&[], &[]).expect("probe");
        let probe = step.selected().expect("selected probe");
        let output = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: probe.action_id().as_str(),
                action_kind: probe.action_kind(),
                max_actions: 0,
                predicted_event_types: probe.event_types(),
                predicted_state_hash: probe.post_state_hash().as_str(),
            },
        )
        .expect("forced novelty");
        assert_eq!(output.entry().action_id(), probe.action_id());
        assert_eq!(output.entry().action_kind(), probe.action_kind());
        assert_eq!(output.entry().event_types(), probe.event_types());
        assert_eq!(output.entry().state_hash(), probe.post_state_hash());
        assert_eq!(output.result()["status"], "horizon");
        assert_eq!(output.result()["acceptedActionCount"], 0);
        assert_eq!(
            output.result()["initialStateHash"],
            serde_json::json!(probe.post_state_hash())
        );
        assert_eq!(
            session.state_hash().expect("hash"),
            probe.post_state_hash().clone()
        );
    }

    #[test]
    fn rejects_a_stale_action_identity() {
        let mut session = opening_session();
        let error = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                action_kind: "mulligan",
                max_actions: 0,
                predicted_event_types: &[],
                predicted_state_hash: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            },
        )
        .expect_err("stale");
        assert!(error.to_string().contains("stale"));
    }

    #[test]
    fn rejects_a_mismatched_prediction() {
        let mut session = opening_session();
        let step = session.probe_novelty(&[], &[]).expect("probe");
        let probe = step.selected().expect("selected probe");
        let error = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: probe.action_id().as_str(),
                action_kind: probe.action_kind(),
                max_actions: 0,
                predicted_event_types: probe.event_types(),
                predicted_state_hash:
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            },
        )
        .expect_err("mismatch");
        assert!(error.to_string().contains("prediction"));
    }

    #[test]
    fn rejects_an_oversize_horizon() {
        let mut session = opening_session();
        let step = session.probe_novelty(&[], &[]).expect("probe");
        let probe = step.selected().expect("selected probe");
        let error = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: probe.action_id().as_str(),
                action_kind: probe.action_kind(),
                max_actions: NOVELTY_ROLLOUT_ACTION_LIMIT + 1,
                predicted_event_types: probe.event_types(),
                predicted_state_hash: probe.post_state_hash().as_str(),
            },
        )
        .expect_err("oversize");
        assert!(error.to_string().contains("maxActions"));
    }
}
