//! Direct proofs for grant-charge-to-ally-this-turn Magic
//! (RULE-CATALOG-0597–0598).
//!
//! Ordinary Charge Magic offers every controlled ally and grants temporary
//! Charge through End Phase. A newly summoned minion can Move and Attack
//! immediately after the grant. With no allied minion in play the cast still
//! resolves as a paid no-op.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn charge() -> Value {
    json!({
        "cardType": "magic",
        "grantChargeToAllyThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn charge_with_ally_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "charge-ally" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-charge-ally-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-charge": charge(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-charge",
                    "north-ally",
                    "north-charge",
                    "north-ally",
                    "north-charge",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn charge_empty_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "charge-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-charge-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charge": charge(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-charge"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid charge session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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

fn has_move_and_attack(session: &Session, unit_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == unit_id
        })
}

fn assert_exact_replay(session: &Session) {
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
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0597_charge_magic_lets_a_summoning_sick_ally_move_and_attack() {
    let encoded = (597..597 + 512)
        .map(charge_with_ally_manifest)
        .find(|candidate| {
            let preview = opening_main(candidate);
            preview
                .legal_actions()
                .expect("ally setup actions")
                .iter()
                .any(|action| {
                    action.descriptor["kind"] == "summon-minion"
                        && action.descriptor["cardId"] == "north-ally"
                })
                && preview
                    .legal_actions()
                    .expect("charge setup actions")
                    .iter()
                    .any(|action| {
                        action.descriptor["kind"] == "cast-magic"
                            && action.descriptor["cardId"] == "north-charge"
                    })
        })
        .expect("bounded seed with summonable ally and Charge after opening");
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-ally"
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned();
    assert!(!has_move_and_attack(&session, &ally_id));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-charge"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert!(event_types(&granted).contains(&"charge-granted"));
    assert!(has_move_and_attack(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0598_charge_magic_grants_charge_to_the_avatar_when_no_minion_is_in_play() {
    let encoded = (598..598 + 256)
        .map(charge_empty_manifest)
        .find(|candidate| {
            Session::new(candidate).is_ok()
                && state(&Session::new(candidate).expect("candidate session"))["players"]["north"]
                    ["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-charge"))
        })
        .expect("bounded seed with Charge in opening hand");
    let mut session = opening_main(&encoded);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(|units| units.iter().all(|unit| unit["kind"] != "minion"))
    );
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned();

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-charge"
            && descriptor["ally"]["kind"] == "avatar"
    });
    assert_eq!(cast["ally"]["instanceId"], avatar_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "charge-granted", "magic-resolved"]
    );
    assert_exact_replay(&session);
}
