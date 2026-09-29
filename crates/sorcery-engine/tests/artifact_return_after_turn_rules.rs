//! Direct proof for the composed bearer-power and end-of-turn Artifact facts.
//!
//! The pinned scenario carries the Artifact, returns it to its owner's hidden hand after the
//! controller's turn, and casts the same physical card again. The admission test proves the
//! bounded bearer-power fact; this scenario proves its composition with the lifecycle effect. The
//! checkpoint and exact replay must retain the same identity.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn returning_artifact() -> Value {
    json!({
        "bearerPowerBonus": 1,
        "cardType": "artifact",
        "manaCost": 0,
        "returnToOwnerHandAfterEachTurn": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn returning_token() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": null,
        "returnToOwnerHandAfterEachTurn": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn multi_return_manifest(seed: u32) -> String {
    let mut value: Value = serde_json::from_str(&manifest(seed)).expect("base manifest JSON");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-bearer");
    value["cards"]["returning-token"] = returning_token();
    value["cards"]["conjure"] = json!({
        "cardType": "magic",
        "effectProgram": { "effects": [
            { "op": "choose-location", "relation": "anywhere" },
            {
                "count": 1,
                "destination": "chosen-location",
                "op": "conjure-token",
                "token": "returning-token",
            }
        ] },
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["north"]["spellbook"] = json!([
        "returning-artifact",
        "conjure",
        "returning-artifact",
        "conjure",
        "returning-artifact",
        "returning-artifact",
    ]);
    value
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("multi-return identity"));
    canonical_json(&value).expect("canonical multi-return manifest")
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-return-after-turn" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-return-after-turn-v2",
        },
        "cards": {
            "returning-artifact": returning_artifact(),
            "north-avatar": avatar(),
            "north-bearer": json!({
                "attack": 1,
                "cardType": "minion",
                "charge": true,
                "defense": 1,
                "manaCost": 0,
                "movementBonus": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-target": json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "returning-artifact", "north-bearer", "returning-artifact",
                    "north-bearer", "returning-artifact", "north-bearer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "returning-artifact", "south-target", "returning-artifact",
                    "south-target", "returning-artifact", "south-target",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

const SCENARIO_SEED: u32 = 4;

fn scenario_manifest() -> String {
    let candidate = manifest(SCENARIO_SEED);
    let opening = state(&Session::new(&candidate).expect("candidate session"));
    let hand = opening["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("opening spellbook hand");
    assert!(
        hand.iter()
            .any(|card| card["cardId"] == "returning-artifact"),
        "pinned scenario seed must deal the returning Artifact"
    );
    assert!(
        hand.iter().any(|card| card["cardId"] == "north-bearer"),
        "pinned scenario seed must deal the bearer"
    );
    let south_hand = opening["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("opening South spellbook hand");
    assert!(
        south_hand
            .iter()
            .any(|card| card["cardId"] == "south-target"),
        "pinned scenario seed must deal the combat target"
    );
    candidate
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action in phase {}",
                state(session)["phase"]
            )
        });
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn composed_bearer_power_and_return_preserve_owner_hand_and_replay_identity() {
    let mut session = Session::new(&scenario_manifest()).expect("valid synthetic manifest");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-bearer"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let bearer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("bearer identity")
        .to_owned();
    let (first_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "returning-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    });
    let artifact_id = first_cast["cardInstanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    let first_realm_entry = state(&session)["realm"]["artifacts"][0]["realmEntry"]
        .as_u64()
        .expect("first realm entry");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == bearer_id.as_str()
            && descriptor["artifactInstanceIds"] == json!([artifact_id.as_str()])
    });
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == bearer_id.as_str())
    );

    let checkpoint = create_game_checkpoint(&session).expect("capture checkpoint");
    let resumed = resume_game_checkpoint(&checkpoint).expect("resume checkpoint");
    assert_eq!(
        resumed.session_hash().expect("resumed session hash"),
        session.session_hash().expect("session hash")
    );

    let (_, end_receipt) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(event_types(&end_receipt).contains(&"turn-ended"));
    assert!(event_types(&end_receipt).contains(&"artifact-returned-to-hand"));
    assert!(state(&session)["realm"].get("artifacts").is_none());
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North hand")
            .iter()
            .any(|card| card["instanceId"] == artifact_id)
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("summoned target")
        .iter()
        .find(|unit| unit["cardId"] == "south-target")
        .expect("target instance")["instanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (second_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "returning-artifact"
            && descriptor["cardInstanceId"] == artifact_id.as_str()
            && descriptor["bearer"]["kind"] == "minion"
            && descriptor["bearer"]["instanceId"] == bearer_id.as_str()
    });
    assert_eq!(second_cast["cardInstanceId"], artifact_id);
    let second_realm_entry = state(&session)["realm"]["artifacts"][0]["realmEntry"]
        .as_u64()
        .expect("second realm entry");
    assert!(second_realm_entry > first_realm_entry);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bearer_id.as_str()
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|step| step["cell"].as_str())
                    .collect::<Vec<_>>()
                    == vec![Some("C4")]
            })
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id.as_str()
    });
    let (_, combat_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend"
            && descriptor["originalTargetParticipates"].as_bool() == Some(true)
    });
    assert!(event_types(&combat_receipt).contains(&"fight-started"));
    let after_combat = state(&session);
    let target = after_combat["realm"]["units"]
        .as_array()
        .expect("combat units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id.as_str())
        .expect("surviving target");
    assert_eq!(target["damage"], 2);
    let bearer = after_combat["realm"]["units"]
        .as_array()
        .expect("combat units")
        .iter()
        .find(|unit| unit["instanceId"] == bearer_id.as_str())
        .expect("damaged bearer before removal");
    assert_eq!(bearer["damage"], 1);

    let (_, removal_receipt) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let removal_types = event_types(&removal_receipt);
    let return_index = removal_types
        .iter()
        .position(|event| *event == "artifact-returned-to-hand")
        .expect("lifecycle return event");
    assert_eq!(return_index, 1);
    assert!(state(&session)["realm"]["artifacts"].is_null());
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("remaining realm units")
            .iter()
            .any(|unit| unit["instanceId"] == bearer_id.as_str())
    );

    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
}

#[test]
#[allow(clippy::too_many_lines)]
fn multiple_loose_returns_and_tokens_follow_storyline_order_and_resume_exactly() {
    let mut session = Session::new(&multi_return_manifest(SCENARIO_SEED)).expect("valid manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });

    let mut ordinary_ids = Vec::new();
    for _ in 0..2 {
        let (cast, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "returning-artifact"
                && descriptor["bearer"].is_null()
                && descriptor["cell"] == "C4"
        });
        ordinary_ids.push(
            cast["cardInstanceId"]
                .as_str()
                .expect("ordinary id")
                .to_owned(),
        );
    }
    let token_cast = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "conjure"
    });
    assert_eq!(token_cast.0["kind"], "cast-magic");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability-location"
            && descriptor["location"] == json!({ "cell": "C4", "region": "surface" })
    });
    let token_id = state(&session)["realm"]["artifacts"]
        .as_array()
        .expect("three loose artifacts")
        .iter()
        .find(|artifact| artifact["source"] == "token")
        .expect("returning token")["instanceId"]
        .as_str()
        .expect("token id")
        .to_owned();
    let mut expected = ordinary_ids
        .iter()
        .map(|id| (id.clone(), "artifact-returned-to-hand"))
        .collect::<Vec<_>>();
    expected.push((token_id.clone(), "artifact-banished"));
    expected.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let return_order = expected
        .iter()
        .rev()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();

    let (_, end_receipt) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(event_types(&end_receipt).contains(&"turn-ended"));
    assert!(
        !event_types(&end_receipt).iter().any(|event| {
            *event == "artifact-returned-to-hand" || *event == "artifact-banished"
        })
    );
    assert_eq!(state(&session)["phase"], "trigger-order");
    let checkpoint = create_game_checkpoint(&session).expect("pending Storyline checkpoint");
    let mut resumed = resume_game_checkpoint(&checkpoint).expect("resume pending Storyline");
    let mut original_receipts = vec![end_receipt];
    let mut resumed_receipts = Vec::new();
    for instance_id in return_order.iter().take(return_order.len() - 1) {
        let predicate = |descriptor: &Value| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == instance_id.as_str()
        };
        let (_, original_receipt) = accept_where(&mut session, predicate);
        let (_, resumed_receipt) = accept_where(&mut resumed, predicate);
        assert_eq!(original_receipt, resumed_receipt);
        original_receipts.push(original_receipt);
        resumed_receipts.push(resumed_receipt);
    }
    let lifecycle_events = original_receipts
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .filter(|event| {
            event.event_type == "artifact-returned-to-hand"
                || event.event_type == "artifact-banished"
        })
        .map(|event| {
            (
                event.payload["instanceId"]
                    .as_str()
                    .expect("event instance id")
                    .to_owned(),
                event.event_type.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let expected_in_order = return_order
        .iter()
        .map(|id| {
            expected
                .iter()
                .find(|(expected_id, _)| expected_id == id)
                .cloned()
                .expect("ordered return source")
        })
        .collect::<Vec<_>>();
    assert_eq!(lifecycle_events, expected_in_order);
    assert_eq!(
        original_receipts
            .iter()
            .flat_map(|receipt| event_types(receipt))
            .filter(|event| *event == "turn-started")
            .count(),
        1
    );
    assert_eq!(
        event_types(original_receipts.last().expect("completed trigger batch")).last(),
        Some(&"turn-started")
    );
    assert!(state(&session)["realm"]["artifacts"].is_null());
    assert_eq!(state(&session), state(&resumed));
    let after_returns = state(&session);
    let north_hand = after_returns["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand");
    for ordinary_id in ordinary_ids {
        assert!(
            north_hand
                .iter()
                .any(|card| card["instanceId"] == ordinary_id)
        );
    }
    assert!(!state(&session).to_string().contains(&token_id));

    let action_ids = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(
        original_receipts.iter().skip(1).collect::<Vec<_>>(),
        resumed_receipts.iter().collect::<Vec<_>>()
    );
}
