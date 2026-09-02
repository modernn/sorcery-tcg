//! Direct proof that a Tower grants power, Ranged, and Spellcaster (RULE-CATALOG-0151).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(seed: u32) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-bolt": {
            "cardType": "magic",
            "damageTargetUnit": 1,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-plain": { "cardType": "site", "elements": ["earth"] },
        "north-tower": { "cardType": "site", "elements": ["earth"], "isTower": true },
        "north-watcher": minion(json!({
            "gainsPowerRangedAndSpellcasterAtopTower": 2,
        })),
        "south-avatar": avatar(),
        "south-site": { "cardType": "site", "elements": ["earth"] },
        "south-target": minion(json!({ "defense": 3, "summonToAnySite": true })),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "tower-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-tower-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-tower",
                    "north-plain",
                    "north-tower",
                    "north-plain",
                    "north-tower",
                    "north-plain",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-watcher",
                    "north-bolt",
                    "north-watcher",
                    "north-bolt",
                    "north-watcher",
                    "north-bolt",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-target"; 6],
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

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

/// Counts the projectile shots and Magic casts the engine credits to one unit.
fn derived_abilities(session: &Session, unit_id: &str) -> (usize, usize) {
    let actions = session.legal_actions().expect("legal actions");
    let shots = actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "shoot-projectile"
                && action.descriptor["shooterInstanceId"] == unit_id
        })
        .count();
    let casts = actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["casterInstanceId"] == unit_id
        })
        .count();
    (shots, casts)
}

fn seeded() -> String {
    (1..=4096)
        .map(manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("tower candidate");
            let opening = state(&preview);
            let north = &opening["players"]["north"];
            ["north-tower", "north-plain"].into_iter().all(|card_id| {
                north["hand"]["atlas"]
                    .as_array()
                    .expect("opening atlas hand")
                    .iter()
                    .any(|card| card["cardId"] == card_id)
            }) && ["north-watcher", "north-bolt"].into_iter().all(|card_id| {
                north["hand"]["spellbook"]
                    .as_array()
                    .expect("opening spellbook hand")
                    .iter()
                    .any(|card| card["cardId"] == card_id)
            })
        })
        .expect("bounded seed with both North site kinds and both North spells in hand")
}

fn play_named_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

#[test]
fn rule_catalog_0151_a_tower_should_grant_power_ranged_and_spellcaster_to_its_occupant() {
    let mut session = Session::new(&seeded()).expect("valid Tower scenario");
    keep(&mut session);
    keep(&mut session);

    play_named_site(&mut session, "north-plain", "C4");
    let grounded = summon(&mut session, "north-watcher", "C4");
    end_and_draw(&mut session);
    play_named_site(&mut session, "south-site", "C1");
    end_and_draw(&mut session);
    play_named_site(&mut session, "north-tower", "C3");
    let elevated = summon(&mut session, "north-watcher", "C3");
    end_and_draw(&mut session);
    // South parks a three-defense minion in the plain watcher's own cell.
    let target = summon(&mut session, "south-target", "C4");
    end_and_draw(&mut session);

    // Only the Tower occupant is credited with Ranged and Spellcaster.
    assert_eq!(derived_abilities(&session, &grounded), (0, 0));
    let (shots, casts) = derived_abilities(&session, &elevated);
    assert!(shots > 0, "the Tower occupant should shoot");
    assert!(casts > 0, "the Tower occupant should cast");

    // Its strike carries the derived power of three, which its printed one could never reach.
    let (_, shot) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == elevated.as_str()
            && descriptor["hit"]["instanceId"] == target.as_str()
    });
    let allocated = shot
        .events
        .iter()
        .find(|event| event.event_type == "strike-damage-allocated")
        .expect("ranged strike allocation");
    assert_eq!(allocated.payload["amount"], 3);
    assert!(
        shot.events
            .iter()
            .any(|event| event.event_type == "minion-died")
    );
    let cleared = state(&session);
    assert!(realm_unit(&cleared, &target).is_none());
    assert!(realm_unit(&cleared, &grounded).is_some());
    assert_exact_replay(&session);
}
