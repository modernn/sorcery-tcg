//! Direct proofs for lure-enemy-minion-one-step-closer Magic (RULE-CATALOG-0601–0602).
//!
//! Lure tempts a nearby enemy minion to take its own closer step toward the
//! caster's ally. Without a qualifying enemy the cast still resolves as a paid
//! no-op.

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

fn enemy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lure() -> Value {
    json!({
        "cardType": "magic",
        "lureEnemyMinionOneStepCloser": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn lure_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lure-enemy" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lure-enemy-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lure": lure(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lure"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
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

fn opening_with_tempted_enemy(encoded: &str) -> (Session, String, String) {
    let mut session = Session::new(encoded).expect("valid lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    end_and_draw(&mut session);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned();
    (session, avatar_id, enemy_id)
}

#[test]
fn rule_catalog_0601_lure_magic_tempts_a_nearby_enemy_one_step_closer() {
    let encoded = (601..601 + 512)
        .map(lure_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-enemy"))
            })
        })
        .expect("bounded seed with a South minion in the opening hand");
    let (mut session, avatar_id, enemy_id) = opening_with_tempted_enemy(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lure"
            && descriptor["temptedEnemy"]["instanceId"] == enemy_id
            && descriptor["ally"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "unit-lured", "magic-resolved"]
    );
    assert_eq!(realm_unit(&state(&session), &enemy_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0602_lure_magic_no_ops_without_a_nearby_enemy_minion() {
    let encoded = lure_manifest(602);
    let mut session = Session::new(&encoded).expect("valid lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let lure_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .find(|card| card["cardId"] == "north-lure")
        .expect("lure in hand")["instanceId"]
        .as_str()
        .expect("lure identity")
        .to_owned();
    let actions: Vec<_> = session
        .legal_actions()
        .expect("lure actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == lure_id
        })
        .collect();
    assert_eq!(actions.len(), 1);
    assert!(actions[0].descriptor.get("temptedEnemy").is_none());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardInstanceId"] == lure_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert_exact_replay(&session);
}
