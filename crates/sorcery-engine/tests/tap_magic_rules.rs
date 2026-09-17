//! Direct proofs for tap-target-minion Magic (RULE-CATALOG-0611–0612).
//!
//! Tap Magic exhausts a ready same-region minion. Enemy Ward absorbs the cast
//! without tapping the minion.

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

fn charger(ward: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
}

fn tap_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "tapTargetMinion": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn tap_manifest(seed: u32, ward: bool) -> String {
    let fixture = if ward { "tap-magic-ward" } else { "tap-magic" };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-tap": tap_spell(),
            "south-avatar": avatar(),
            "south-charger": charger(ward),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-tap"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-charger"; 6],
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
    let mut session = Session::new(encoded).expect("valid tap magic session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
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

fn stage_south_charger(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-charger"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let charger_id = summoned["cardInstanceId"]
        .as_str()
        .expect("South charger identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    charger_id
}

#[test]
fn rule_catalog_0611_tap_magic_exhausts_a_ready_minion() {
    let encoded = tap_manifest(611, false);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&before, &charger_id)["warded"], false);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-tap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == charger_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-tapped", "magic-resolved"]
    );
    let tapped = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-tapped")
        .expect("tap event");
    assert_eq!(tapped.payload["instanceId"], charger_id);
    assert_eq!(
        tapped.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["tapped"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0612_tap_magic_is_absorbed_by_enemy_ward() {
    let encoded = tap_manifest(612, true);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&before, &charger_id)["warded"], true);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-tap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == charger_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-tapped")
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&after, &charger_id)["warded"], false);
    assert_exact_replay(&session);
}
