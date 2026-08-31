//! Typed legal-action contract shared by engine consumers.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::canonical::{CanonicalError, IdentityHash, canonical_json, identity_hash};

/// A player seat.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Seat {
    /// North player.
    North,
    /// South player.
    South,
}

/// A version-bound request referencing an engine-issued action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequest {
    /// Opaque action identity as supplied by the caller. Unknown strings are
    /// preserved in rejected-attempt journals.
    pub action_id: String,
    /// Acting seat.
    pub seat: Seat,
    /// State version that issued the action.
    pub state_version: u64,
}

/// An engine-issued legal action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalAction {
    /// Opaque action identity.
    pub action_id: IdentityHash,
    /// Typed action payload at the external boundary.
    pub descriptor: Value,
    /// Human-readable action label.
    pub label: String,
    /// Acting seat.
    pub seat: Seat,
    /// State version that issued the action.
    pub state_version: u64,
}

/// A rejected action category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    /// The request targets an earlier or later state version.
    StaleVersion,
    /// The game has already finished.
    TerminalState,
    /// The action identity was not issued for the current state.
    UnknownAction,
    /// The request names the non-deciding seat.
    WrongSeat,
}

/// A stable rejection returned without changing authoritative game state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    /// Stable machine-readable category.
    pub code: RejectionCode,
    /// Hash of the unchanged authoritative state.
    pub current_state_hash: IdentityHash,
    /// Version of the unchanged authoritative state.
    pub current_state_version: u64,
    /// Stable user-facing explanation.
    pub message: String,
}

/// One accepted or rejected request audit record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    /// One-based attempt sequence.
    pub attempt_sequence: u64,
    /// Hash of the state against which the request was checked.
    pub authoritative_state_hash: IdentityHash,
    /// Version against which the request was checked.
    pub authoritative_state_version: u64,
    /// Stable accepted/rejected outcome.
    pub outcome: AttemptOutcome,
    /// Rejection category, when rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<RejectionCode>,
    /// Receipt identity, when accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<IdentityHash>,
    /// Original typed request.
    pub request: ActionRequest,
}

/// Stable attempt outcome.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AttemptOutcome {
    /// The request produced a receipt.
    Accepted,
    /// The request was rejected.
    Rejected,
}

/// The action and receipt that caused an event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventCause {
    /// Opaque action identity.
    pub action_id: IdentityHash,
    /// One-based receipt sequence.
    pub receipt_sequence: u64,
}

/// One authoritative game event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Receipt cause.
    pub cause: EventCause,
    /// Canonical event identity.
    pub event_id: IdentityHash,
    /// One-based global event sequence.
    pub event_sequence: u64,
    /// Typed event data at the external boundary.
    pub payload: Value,
    /// Stable event category.
    #[serde(rename = "type")]
    pub event_type: String,
}

/// An authoritative accepted-action receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    /// Accepted action identity.
    pub action_id: IdentityHash,
    /// Events caused by the action.
    pub events: Vec<Event>,
    /// Resulting state version.
    pub next_state_version: u64,
    /// Resulting state hash.
    pub post_state_hash: IdentityHash,
    /// Previous state hash.
    pub pre_state_hash: IdentityHash,
    /// Action-time random draw records.
    pub random_draws: Vec<Value>,
    /// Canonical receipt identity.
    pub receipt_id: IdentityHash,
    /// One-based receipt sequence.
    pub receipt_sequence: u64,
    /// Acting seat.
    pub seat: Seat,
    /// Previous state version.
    pub state_version: u64,
}

/// Receipt data before its self-identity is attached.
pub struct ReceiptInput {
    /// Accepted action identity.
    pub action_id: IdentityHash,
    /// Events caused by the action.
    pub events: Vec<Event>,
    /// Resulting state version.
    pub next_state_version: u64,
    /// Resulting state hash.
    pub post_state_hash: IdentityHash,
    /// Previous state hash.
    pub pre_state_hash: IdentityHash,
    /// Action-time random draw records.
    pub random_draws: Vec<Value>,
    /// One-based receipt sequence.
    pub receipt_sequence: u64,
    /// Acting seat.
    pub seat: Seat,
    /// Previous state version.
    pub state_version: u64,
}

/// Binds an action descriptor to its contract, seat, and state version.
///
/// # Errors
///
/// Returns [`CanonicalError`] when the descriptor cannot be canonicalized.
pub fn opaque_action_id(
    contract: &str,
    seat: Seat,
    state_version: u64,
    descriptor: &Value,
) -> Result<IdentityHash, CanonicalError> {
    identity_hash(&json!({
        "contract": contract,
        "descriptor": descriptor,
        "seat": seat,
        "stateVersion": state_version,
    }))
}

/// Returns the canonical action order without repeatedly serializing sort keys.
///
/// # Errors
///
/// Returns [`CanonicalError`] when a descriptor cannot be canonicalized.
pub fn order_legal_actions(actions: Vec<LegalAction>) -> Result<Vec<LegalAction>, CanonicalError> {
    let mut keyed = actions
        .into_iter()
        .map(|action| Ok((canonical_json(&action.descriptor)?, action)))
        .collect::<Result<Vec<_>, CanonicalError>>()?;
    keyed.sort_unstable_by(|(left_descriptor, left), (right_descriptor, right)| {
        left_descriptor
            .cmp(right_descriptor)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    Ok(keyed.into_iter().map(|(_, action)| action).collect())
}

/// Creates stable authoritative events from typed outcome pairs.
///
/// # Errors
///
/// Returns [`CanonicalError`] when an event body cannot be canonicalized.
pub fn create_events(
    action_id: &IdentityHash,
    receipt_sequence: u64,
    first_event_sequence: u64,
    outcomes: &[(String, Value)],
) -> Result<Vec<Event>, CanonicalError> {
    outcomes
        .iter()
        .enumerate()
        .map(|(index, (event_type, payload))| {
            let event_sequence = first_event_sequence + index as u64;
            let cause = EventCause {
                action_id: action_id.clone(),
                receipt_sequence,
            };
            let event_id = identity_hash(&json!({
                "cause": cause,
                "eventSequence": event_sequence,
                "payload": payload,
                "type": event_type,
            }))?;
            Ok(Event {
                cause,
                event_id,
                event_sequence,
                payload: payload.clone(),
                event_type: event_type.clone(),
            })
        })
        .collect()
}

/// Creates a self-identifying authoritative receipt.
///
/// # Errors
///
/// Returns [`CanonicalError`] when the receipt body cannot be canonicalized.
pub fn create_receipt(input: ReceiptInput) -> Result<Receipt, CanonicalError> {
    let receipt_id = identity_hash(&json!({
        "actionId": input.action_id,
        "events": input.events,
        "nextStateVersion": input.next_state_version,
        "postStateHash": input.post_state_hash,
        "preStateHash": input.pre_state_hash,
        "randomDraws": input.random_draws,
        "receiptSequence": input.receipt_sequence,
        "seat": input.seat,
        "stateVersion": input.state_version,
    }))?;
    Ok(Receipt {
        action_id: input.action_id,
        events: input.events,
        next_state_version: input.next_state_version,
        post_state_hash: input.post_state_hash,
        pre_state_hash: input.pre_state_hash,
        random_draws: input.random_draws,
        receipt_id,
        receipt_sequence: input.receipt_sequence,
        seat: input.seat,
        state_version: input.state_version,
    })
}

/// Creates a stable rejection response.
#[must_use]
pub fn create_rejection(
    code: RejectionCode,
    current_state_version: u64,
    current_state_hash: IdentityHash,
) -> Rejection {
    let message = match code {
        RejectionCode::StaleVersion => "That action belongs to an earlier game state.",
        RejectionCode::TerminalState => "The game is already over.",
        RejectionCode::UnknownAction => "That action is not available.",
        RejectionCode::WrongSeat => "That action belongs to the other seat.",
    }
    .to_owned();
    Rejection {
        code,
        current_state_hash,
        current_state_version,
        message,
    }
}

/// Creates an accepted request audit record.
#[must_use]
pub fn accepted_attempt(
    attempt_sequence: u64,
    request: ActionRequest,
    authoritative_state_version: u64,
    authoritative_state_hash: IdentityHash,
    receipt_id: IdentityHash,
) -> Attempt {
    Attempt {
        attempt_sequence,
        authoritative_state_hash,
        authoritative_state_version,
        outcome: AttemptOutcome::Accepted,
        reason_code: None,
        receipt_id: Some(receipt_id),
        request,
    }
}

/// Creates a rejected request audit record.
#[must_use]
pub fn rejected_attempt(
    attempt_sequence: u64,
    request: ActionRequest,
    authoritative_state_version: u64,
    authoritative_state_hash: IdentityHash,
    reason_code: RejectionCode,
) -> Attempt {
    Attempt {
        attempt_sequence,
        authoritative_state_hash,
        authoritative_state_version,
        outcome: AttemptOutcome::Rejected,
        reason_code: Some(reason_code),
        receipt_id: None,
        request,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LegalAction, Seat, opaque_action_id, order_legal_actions};

    fn action(descriptor: serde_json::Value) -> LegalAction {
        LegalAction {
            action_id: opaque_action_id("test-v1", Seat::North, 3, &descriptor)
                .expect("valid action identity"),
            label: descriptor["kind"].as_str().expect("action kind").to_owned(),
            descriptor,
            seat: Seat::North,
            state_version: 3,
        }
    }

    #[test]
    fn opaque_action_id_should_match_existing_contract_vector() {
        let descriptor = json!({ "kind": "pass" });

        assert_eq!(
            opaque_action_id("test-v1", Seat::North, 3, &descriptor)
                .expect("valid action identity")
                .to_string(),
            "sha256:c6a0779245e80ea05b708f13f579f46c4f0674514b88cab86e8f3b1441ba0a5a"
        );
    }

    #[test]
    fn legal_actions_should_match_existing_descriptor_order() {
        let ordered = order_legal_actions(vec![
            action(json!({ "kind": "pass" })),
            action(json!({ "kind": "draw", "zone": "atlas" })),
        ])
        .expect("valid legal actions");

        assert_eq!(ordered[0].descriptor["kind"], "draw");
        assert_eq!(ordered[1].descriptor["kind"], "pass");
    }
}
