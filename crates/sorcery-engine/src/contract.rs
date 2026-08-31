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
    /// Opaque action identity.
    pub action_id: IdentityHash,
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
