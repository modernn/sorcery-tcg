//! Direct proofs that Fate Lose strips Tower bonuses (RULE-CATALOG-0335–0336).
//!
//! Official Fate is a Lose effect: a covered non-Ordinary Tower has no printed
//! abilities, including the bonus that grants power, Ranged, and Spellcaster.
//! Ordinary Towers in the same 2x2 keep that bonus. Fate can cover voids, so
//! these proofs play the Tower onto a covered empty cell and never stand a
//! minion there first.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn fate() -> Value {
    json!({
        "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn watcher() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "gainsPowerRangedAndSpellcasterAtopTower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bolt() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tower(ordinary: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
        "isTower": true,
    });
    if ordinary {
        value["ordinary"] = json!(true);
    }
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32, ordinary_tower: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "fate-tower-lose",
                "ordinary": ordinary_tower,
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": if ordinary_tower {
                "synthetic-fate-tower-ordinary-v1"
            } else {
                "synthetic-fate-tower-v1"
            },
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bolt": bolt(),
            "north-fate": fate(),
            "north-open": site(),
            "north-tower": tower(ordinary_tower),
            "north-watcher": watcher(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-tower",
                    "north-open",
                    "north-tower",
                    "north-open",
                    "north-tower",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-fate",
                    "north-watcher",
                    "north-bolt",
                    "north-fate",
                    "north-watcher",
                    "north-bolt",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
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

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn state(session: &Session) -> Value {
    session.public_view(Seat::North).expect("public view")
}

fn fate_covers_c3_not_c4(descriptor: &Value) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == "north-fate"
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C3"))
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().all(|value| value != "C4"))
}

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

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn opening(ordinary_tower: bool) -> Session {
    (1..=4096)
        .map(|seed| manifest(seed, ordinary_tower))
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate Tower candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-tower".to_owned())
                && spells.contains(&"north-fate".to_owned())
                && spells.contains(&"north-watcher".to_owned())
                && spells.contains(&"north-bolt".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Fate, a Tower, a watcher, and a bolt")
}

fn watcher_atop_covered_tower(ordinary_tower: bool) -> (Session, String) {
    let mut session = opening(ordinary_tower);
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, fate_covers_c3_not_c4);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-tower"
            && descriptor["cell"] == "C3"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-watcher"
            && descriptor["cell"] == "C3"
    });
    let watcher_id = summoned["cardInstanceId"]
        .as_str()
        .expect("watcher identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-dummy"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (session, watcher_id)
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> &'a Value {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("watcher remains in play")
}

#[test]
fn rule_catalog_0335_fate_strips_tower_bonus_from_a_non_ordinary_site() {
    let (session, watcher_id) = watcher_atop_covered_tower(false);
    let current = state(&session);
    let occupant = realm_unit(&current, &watcher_id);
    assert_eq!(
        occupant["attack"], 1,
        "Fate Lose strips the Tower power bonus"
    );
    assert_eq!(occupant["defense"], 1);
    assert_eq!(
        derived_abilities(&session, &watcher_id),
        (0, 0),
        "Fate Lose strips the Tower Ranged and Spellcaster grants"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0336_ordinary_tower_keeps_its_bonus_under_fate() {
    let (session, watcher_id) = watcher_atop_covered_tower(true);
    let current = state(&session);
    let occupant = realm_unit(&current, &watcher_id);
    assert_eq!(
        occupant["attack"], 3,
        "an Ordinary Tower still grants +2 power"
    );
    assert_eq!(occupant["defense"], 3);
    let (shots, casts) = derived_abilities(&session, &watcher_id);
    assert!(shots > 0, "an Ordinary Tower still grants Ranged");
    assert!(casts > 0, "an Ordinary Tower still grants Spellcaster");
    assert_exact_replay(&session);
}
