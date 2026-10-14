//! Forced engine-issued action plus novelty rollout from the resulting snapshot.

use serde_json::{Value, json};

use crate::canonical::IdentityHash;
use crate::checkpoint::GameCheckpoint;
use crate::contract::{ActionRequest, RejectionCode};
use crate::novelty_rollout::{
    NOVELTY_ROLLOUT_ACTION_LIMIT, NoveltyRolloutOutput, run_novelty_rollout,
};
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
/// Every error leaves the caller's session unchanged.
///
/// # Errors
///
/// Returns [`SessionError`] when the action is stale, the prediction does not replay,
/// or novelty materialization fails.
pub fn run_novelty_from_forced_action(
    session: &mut Session,
    input: ForcedNoveltyInput<'_>,
) -> Result<ForcedNoveltyOutput, SessionError> {
    let predicted_hash = IdentityHash::parse(input.predicted_state_hash)
        .map_err(|_| dispatch_error("forced novelty predictedStateHash is invalid"))?;
    if input.max_actions > NOVELTY_ROLLOUT_ACTION_LIMIT {
        return Err(dispatch_error(&format!(
            "maxActions must be 0-{NOVELTY_ROLLOUT_ACTION_LIMIT}"
        )));
    }
    let mut branch = session.clone();
    let mut issued = branch
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
    let receipt = match branch.step(request)? {
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
    let state_hash = branch.state_hash()?;
    if state_hash != predicted_hash || event_types != input.predicted_event_types {
        return Err(dispatch_error(
            "forced novelty prediction did not replay exactly",
        ));
    }
    let rollout = run_novelty_rollout(&branch, input.max_actions)?;
    if rollout.result().get("initialStateHash") != Some(&json!(predicted_hash)) {
        return Err(dispatch_error(
            "forced novelty rollout started from the wrong state",
        ));
    }
    *session = branch;
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
    use crate::canonical::{canonical_json, identity_hash};
    use crate::contract::ActionRequest;
    use crate::novelty_rollout::NOVELTY_ROLLOUT_ACTION_LIMIT;
    use crate::session::{Session, StepResult};
    use crate::session_json::{RpcRequest, SessionJsonService};
    use crate::synthetic::synthetic_demo_session;
    use serde_json::{Value, json};

    fn opening_session() -> Session {
        synthetic_demo_session(31)
    }

    fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) {
        let action = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .unwrap();
        assert!(matches!(
            session.step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            }),
            Ok(StepResult::Accepted(_))
        ));
    }

    fn unsupported_forced_action_session(ward_applied: bool) -> Session {
        let avatar = json!({
            "attack": 1, "defense": 1, "cardType": "avatar", "drawSpell": false, "life": 20,
        });
        let site = json!({ "cardType": "site", "elements": ["earth"] });
        let threshold = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
        let mut manifest = json!({
            "authority": {
                "contentHash": identity_hash(&json!({ "fixture": "forced-novelty-unsupported" })).unwrap(),
                "mode": "synthetic", "revisionId": "forced-novelty-unsupported-v1",
            },
            "cards": {
                "north-avatar": avatar, "south-avatar": avatar,
                "north-site": site, "south-site": site,
                "north-minion": {
                    "attack": 4, "defense": 4, "cardType": "minion", "manaCost": 0,
                    "takesLessDamage": 1, "thresholds": threshold,
                },
                "north-ward": {
                    "cardType": "magic", "manaCost": 0, "grantWardToTargetMinion": true,
                    "thresholds": threshold,
                },
                "north-rain": {
                    "cardType": "magic", "manaCost": 1, "damageEachAbovegroundMinion": 1,
                    "thresholds": threshold,
                },
                "south-minion": {
                    "attack": 1, "defense": 1, "cardType": "minion", "manaCost": 0,
                    "thresholds": threshold,
                },
            },
            "decks": {
                "north": {
                    "atlas": vec!["north-site"; 5], "avatar": "north-avatar",
                    "spellbook": ["north-minion", "north-ward", "north-rain"],
                },
                "south": {
                    "atlas": vec!["south-site"; 5], "avatar": "south-avatar",
                    "spellbook": vec!["south-minion"; 5],
                },
            },
            "engineVersion": "sorcery-core-v1", "firstSeat": "north",
            "schemaVersion": 1, "seed": 31,
        });
        manifest["manifestId"] = json!(identity_hash(&manifest).unwrap());
        let mut session = Session::new(&canonical_json(&manifest).unwrap()).unwrap();
        for _ in 0..2 {
            accept_where(&mut session, |action| {
                action["kind"] == "mulligan"
                    && action["atlasOrder"] == json!([])
                    && action["spellbookOrder"] == json!([])
            });
        }
        accept_where(&mut session, |action| {
            action["kind"] == "play-site" && action["cell"] == "C4"
        });
        accept_where(&mut session, |action| action["kind"] == "summon-minion");
        if ward_applied {
            accept_where(&mut session, |action| {
                action["kind"] == "cast-magic" && action["cardId"] == "north-ward"
            });
        }
        session
    }

    fn rpc(service: &mut SessionJsonService, method: &str, params: &Value) -> Value {
        let request: RpcRequest = serde_json::from_value(json!({
            "schemaVersion": 1, "id": 1, "method": method, "params": params,
        }))
        .unwrap();
        serde_json::to_value(service.handle(&request)).unwrap()
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
    fn invalid_forced_predictions_leave_complete_session_unchanged() {
        let root = opening_session();
        let probe = root.probe_novelty(&[], &[]).unwrap();
        let probe = probe.selected().unwrap();
        for (id, kind, hash, events, horizon) in [
            (
                probe.action_id().as_str(),
                probe.action_kind(),
                "not-a-hash",
                probe.event_types(),
                0,
            ),
            (
                probe.action_id().as_str(),
                probe.action_kind(),
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                probe.event_types(),
                0,
            ),
            (
                probe.action_id().as_str(),
                probe.action_kind(),
                probe.post_state_hash().as_str(),
                &[][..],
                0,
            ),
            (
                probe.action_id().as_str(),
                probe.action_kind(),
                probe.post_state_hash().as_str(),
                probe.event_types(),
                NOVELTY_ROLLOUT_ACTION_LIMIT + 1,
            ),
            (
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                probe.action_kind(),
                probe.post_state_hash().as_str(),
                probe.event_types(),
                0,
            ),
            (
                probe.action_id().as_str(),
                "wrong-kind",
                probe.post_state_hash().as_str(),
                probe.event_types(),
                0,
            ),
        ] {
            let mut session = root.clone();
            let before = crate::checkpoint::create_game_checkpoint(&session).unwrap();
            let exported = session.replay_value().unwrap();
            assert!(
                run_novelty_from_forced_action(
                    &mut session,
                    ForcedNoveltyInput {
                        action_id: id,
                        action_kind: kind,
                        max_actions: horizon,
                        predicted_event_types: events,
                        predicted_state_hash: hash,
                    }
                )
                .is_err()
            );
            assert_eq!(
                crate::checkpoint::create_game_checkpoint(&session).unwrap(),
                before
            );
            assert_eq!(session.replay_value().unwrap(), exported);
            assert_eq!(
                session.legal_actions().unwrap(),
                root.legal_actions().unwrap()
            );
            assert_eq!(session.unsupported_mechanic(), None);
        }
    }

    #[test]
    fn unsupported_forced_step_aborts_only_the_discarded_branch() {
        let mut session = unsupported_forced_action_session(true);
        let action = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| action.descriptor["cardId"] == "north-rain")
            .unwrap();
        let checkpoint = crate::checkpoint::create_game_checkpoint(&session).unwrap();
        let replay = session.replay_value().unwrap();
        let error = run_novelty_from_forced_action(
            &mut session,
            ForcedNoveltyInput {
                action_id: action.action_id.as_str(),
                action_kind: action.descriptor["kind"].as_str().unwrap(),
                max_actions: 0,
                predicted_event_types: &[],
                predicted_state_hash:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            },
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("ordering Ward and other damage prevention")
        );
        assert_eq!(session.unsupported_mechanic(), None);
        assert_eq!(
            crate::checkpoint::create_game_checkpoint(&session).unwrap(),
            checkpoint
        );
        assert_eq!(session.replay_value().unwrap(), replay);
        assert!(session.legal_actions().unwrap().contains(&action));
    }

    #[test]
    fn rpc_error_preserves_issued_action_and_persistent_session() {
        let mut service = SessionJsonService::new();
        let manifest = crate::synthetic::synthetic_demo_manifest_json(31).unwrap();
        assert!(rpc(&mut service, "new", &json!({ "manifestJson": manifest }))["error"].is_null());
        let probe = rpc(
            &mut service,
            "probeNovelty",
            &json!({ "committedActionKinds": [], "committedEventTypes": [] }),
        );
        let index = usize::try_from(probe["result"]["selectedIndex"].as_u64().unwrap()).unwrap();
        let selected = probe["result"]["probes"][index].clone();
        let before = rpc(&mut service, "exportSession", &json!({}))["result"].clone();
        let failure = rpc(
            &mut service,
            "runNoveltyFromForcedAction",
            &json!({
                "actionId": selected["actionId"],
                "actionKind": selected["actionKind"],
                "maxActions": 501,
                "predictedEventTypes": selected["eventTypes"],
                "predictedStateHash": selected["postStateHash"],
            }),
        );
        assert!(
            failure["error"]["message"]
                .as_str()
                .unwrap()
                .contains("maxActions")
        );
        assert_eq!(
            rpc(&mut service, "exportSession", &json!({}))["result"],
            before
        );
        let accepted = rpc(
            &mut service,
            "step",
            &json!({
                "actionId": selected["actionId"],
                "seat": "north",
                "stateVersion": 0,
            }),
        );
        assert_eq!(accepted["result"]["accepted"], true);
    }

    #[test]
    fn failed_rollout_report_still_commits_only_the_valid_forced_action() {
        let root = unsupported_forced_action_session(false);
        let action = root
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| action.descriptor["cardId"] == "north-ward")
            .unwrap();
        let mut direct = root.clone();
        let receipt = match direct
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .unwrap()
        {
            StepResult::Accepted(receipt) => receipt,
            StepResult::Rejected(_) => panic!("issued ward action rejected"),
        };
        let mut events = receipt
            .events
            .iter()
            .map(|event| event.event_type.clone())
            .collect::<Vec<_>>();
        events.sort();
        events.dedup();
        let state_hash = direct.state_hash().unwrap();
        let mut dispatched = root;
        let output = run_novelty_from_forced_action(
            &mut dispatched,
            ForcedNoveltyInput {
                action_id: action.action_id.as_str(),
                action_kind: action.descriptor["kind"].as_str().unwrap(),
                max_actions: 1,
                predicted_event_types: &events,
                predicted_state_hash: state_hash.as_str(),
            },
        )
        .unwrap();
        assert_eq!(output.result()["status"], "failed");
        assert_eq!(output.result()["rulesCoverage"], "unranked_partial_rules");
        assert_eq!(
            dispatched.replay_value().unwrap(),
            direct.replay_value().unwrap()
        );
    }

    #[test]
    fn successful_dispatch_matches_one_direct_step_with_nonzero_rollout() {
        let root = opening_session();
        let probe = root.probe_novelty(&[], &[]).unwrap();
        let probe = probe.selected().unwrap();
        let action = root
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| action.action_id == *probe.action_id())
            .unwrap();
        let mut direct = root.clone();
        assert!(matches!(
            direct.step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            }),
            Ok(StepResult::Accepted(_))
        ));
        let mut dispatched = root;
        let output = run_novelty_from_forced_action(
            &mut dispatched,
            ForcedNoveltyInput {
                action_id: probe.action_id().as_str(),
                action_kind: probe.action_kind(),
                max_actions: 1,
                predicted_event_types: probe.event_types(),
                predicted_state_hash: probe.post_state_hash().as_str(),
            },
        )
        .unwrap();
        assert_eq!(
            output.result()["initialStateHash"],
            json!(probe.post_state_hash())
        );
        assert_eq!(
            dispatched.replay_value().unwrap(),
            direct.replay_value().unwrap()
        );
        assert_eq!(
            dispatched.legal_actions().unwrap(),
            direct.legal_actions().unwrap()
        );
        assert_eq!(
            crate::checkpoint::create_game_checkpoint(&dispatched).unwrap(),
            crate::checkpoint::create_game_checkpoint(&direct).unwrap()
        );
        let next = direct.legal_actions().unwrap().into_iter().next().unwrap();
        let request = ActionRequest {
            action_id: next.action_id.to_string(),
            seat: next.seat,
            state_version: next.state_version,
        };
        assert_eq!(
            dispatched.step(request.clone()).unwrap(),
            direct.step(request).unwrap()
        );
        assert_eq!(
            dispatched.replay_value().unwrap(),
            direct.replay_value().unwrap()
        );
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
