//! Direct proofs for damage-random-unit-at-location Magic (RULE-CATALOG-0607–0608, 1027).
//!
//! Lightning Bolt-style Magic picks a surface location, then deterministically
//! damages one random unit there. Ward absorbs the hit when the random draw
//! selects a warded minion.
//!
//! 1027 covers damage-random-unit-at-location Magic killing a Deathrite minion:
//! the controller draws a site and magic-resolved only appears after deathrite
//! settlement.

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

fn deathrite_minion() -> Value {
    minion(json!({
        "deathriteDrawSite": true,
        "defense": 1,
    }))
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

fn bolt_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "random-bolt-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-random-bolt-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bolt": bolt(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
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

fn seed_with_deathrite(start: u32) -> String {
    (start..start + 256)
        .map(bolt_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-bolt"))
            })
        })
        .expect("bounded seed with random bolt Deathrite setup")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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

#[test]
fn rule_catalog_1027_random_bolt_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_deathrite(1027);
    let mut session = opening_main(&encoded);
    let target_id = one_minion_at_c2(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

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
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(receipt.events[1].payload["targetInstanceId"], target_id);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    let types = event_types(&receipt);
    let damage_dealt = types
        .iter()
        .position(|event_type| *event_type == "damage-dealt")
        .expect("damage-dealt index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected damage-dealt, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert!(realm_unit(&finished, &target_id).is_none());
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}
