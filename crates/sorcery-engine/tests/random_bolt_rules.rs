//! Direct proofs for damage-random-unit-at-location Magic (RULE-CATALOG-0607–0608).
//!
//! Lightning Bolt-style Magic picks a surface location, then deterministically
//! damages one random unit there. Ward absorbs the hit when the random draw
//! selects a warded minion.

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn bolt() -> Value {
    json!({
        "cardType": "magic",
        "damageRandomUnitAtLocation": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn bolt_manifest(seed: u32, ward: bool) -> String {
    let south_minion = if ward {
        minion(json!({ "ward": true }))
    } else {
        minion(json!({}))
    };
    let fixture = if ward {
        "random-bolt-ward"
    } else {
        "random-bolt"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bolt": bolt(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": south_minion,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bolt"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
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
    let mut session = Session::new(encoded).expect("valid random bolt session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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

fn two_minions_at_c2(session: &mut Session) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let mut candidates = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C2"
                && descriptor["region"].is_null()
        });
        candidates.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned candidate identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    candidates.sort_unstable();
    candidates
}

fn one_minion_at_c2(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C2"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned()
}

#[test]
fn rule_catalog_0607_random_bolt_damages_one_random_unit_at_the_chosen_location() {
    let encoded = bolt_manifest(607, false);
    let mut session = opening_main(&encoded);
    let candidates = two_minions_at_c2(&mut session);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bolt"
            && descriptor["targetLocation"]["cell"] == "C2"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    assert!(receipt.random_draws.iter().all(|draw| {
        draw["purpose"] == "magic_random_unit_at_location"
            && draw["domain"]["kind"] == "unit_index_candidate"
            && draw["domain"]["exclusiveMaximum"] == 2
    }));
    let accepted = receipt
        .random_draws
        .iter()
        .rev()
        .find(|draw| draw["domain"]["accepted"] == true)
        .expect("accepted random draw");
    let selected_index = usize::try_from(
        accepted["result"].as_u64().expect("random uint32") % candidates.len() as u64,
    )
    .expect("candidate index");
    let selected = candidates[selected_index].clone();
    let spared = candidates
        .iter()
        .find(|instance_id| **instance_id != selected)
        .expect("spared candidate")
        .clone();
    assert_eq!(
        receipt.events[1].payload["targetInstanceId"],
        selected.as_str()
    );

    let damaged = state(&session);
    assert_eq!(
        realm_unit(&damaged, &selected).expect("struck unit")["damage"],
        2
    );
    assert_eq!(
        realm_unit(&damaged, &spared).expect("spared unit")["damage"],
        0
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0608_random_bolt_lets_ward_absorb_the_random_hit() {
    let encoded = bolt_manifest(608, true);
    let mut session = opening_main(&encoded);
    let target_id = one_minion_at_c2(&mut session);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bolt"
            && descriptor["targetLocation"]["cell"] == "C2"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "ward-broken",
            "magic-resolved",
        ]
    );
    let snapshot = state(&session);
    let warded = realm_unit(&snapshot, &target_id).expect("warded survivor");
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    assert_exact_replay(&session);
}
