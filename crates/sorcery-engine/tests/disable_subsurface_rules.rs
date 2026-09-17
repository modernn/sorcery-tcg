//! Direct proofs for Disable settling a subsurface burrower
//! (RULE-CATALOG-0026, RULE-CATALOG-0698, RULE-CATALOG-1127, RULE-CATALOG-1139).
//!
//! Freeze (`disableTargetNearbyMinionUntilNextTurn`) kills an underground
//! Burrowing minion because Disable drops Burrowing and region settlement then
//! removes the stranded unit. A surface Burrowing minion stays in play,
//! disabled but alive. Distinct from 0661–0662, which Freeze a surface minion
//! until the next Start Phase or absorb into Ward.

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

fn burrower(spellcaster: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if spellcaster {
        value["spellcaster"] = json!(true);
    }
    value
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn underground_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "disable-subsurface-underground" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-disable-subsurface-underground-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-caster": burrower(true),
            "south-freeze": freeze(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-freeze",
                    "south-caster",
                    "south-freeze",
                    "south-caster",
                    "south-freeze",
                    "south-caster",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn surface_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "disable-subsurface-surface" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-disable-subsurface-surface-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-burrower": burrower(false),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-freeze"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-burrower"; 6],
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
    let mut session = Session::new(encoded).expect("valid Disable subsurface session");
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

fn south_opening_has(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn seed_underground() -> String {
    (698..698 + 512)
        .map(underground_manifest)
        .find(|candidate| south_opening_has(candidate, &["south-freeze", "south-caster"]))
        .expect("bounded seed with Freeze and a Burrowing spellcaster")
}

fn south_summons_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Burrowing minion identity")
        .to_owned()
}

#[test]
fn rule_catalog_0698_disable_kills_an_underground_burrowing_target() {
    let encoded = seed_underground();
    let mut session = opening_main(&encoded);
    let target_id = south_summons_at(&mut session, "south-caster", "C1");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let bury_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == bury_id
            && descriptor["target"]["instanceId"] == target_id
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let freeze_id = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .iter()
        .find(|card| card["cardId"] == "south-freeze")
        .expect("Freeze in hand")["instanceId"]
        .as_str()
        .expect("Freeze identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == freeze_id
            && descriptor["casterInstanceId"] == target_id
            && descriptor["target"]["instanceId"] == target_id
    });

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-disabled",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1127_disable_leaves_surface_burrowing_minion_in_play() {
    let encoded = surface_manifest(26);
    let mut session = opening_main(&encoded);
    let target_id = south_summons_at(&mut session, "south-burrower", "C4");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let before = state(&session);
    assert_eq!(
        realm_unit(&before, &target_id).expect("surface burrower")["region"],
        "surface"
    );
    assert!(realm_unit(&before, &target_id).expect("surface burrower")["disableEffects"].is_null());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let after = state(&session);
    let unit = realm_unit(&after, &target_id).expect("Disabled surface burrower");
    assert_eq!(unit["region"], "surface");
    assert!(!unit["disableEffects"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1139_freeze_leaves_surface_burrowing_minion_in_play() {
    let encoded = surface_manifest(26);
    let mut session = opening_main(&encoded);
    let target_id = south_summons_at(&mut session, "south-burrower", "C4");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let before = state(&session);
    assert_eq!(
        realm_unit(&before, &target_id).expect("surface burrower")["region"],
        "surface"
    );
    assert!(realm_unit(&before, &target_id).expect("surface burrower")["disableEffects"].is_null());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let after = state(&session);
    let unit = realm_unit(&after, &target_id).expect("Frozen surface burrower");
    assert_eq!(unit["region"], "surface");
    assert!(!unit["disableEffects"].is_null());
    assert_exact_replay(&session);
}
