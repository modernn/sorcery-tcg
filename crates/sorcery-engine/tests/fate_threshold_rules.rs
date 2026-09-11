//! Direct proofs for Fate stripping power-threshold site entry (RULE-CATALOG-0327–0328).
//!
//! Official Fate is a Lose effect: a covered non-Ordinary site has no printed
//! abilities. A power-threshold bar is a printed site ability, so Fate lifts it
//! for movement, teleport, and free placement. Ordinary sites in the same 2x2
//! keep the bar. Fate Genesis submerges occupants of affected sites, so these
//! proofs cover the threshold site without standing a minion on it first.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn blocked_site(ordinary: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
        "preventsUnitsWithPowerAtLeastFromEntering": 3,
    });
    if ordinary {
        value["ordinary"] = json!(true);
    }
    value
}

fn heavy() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fate() -> Value {
    json!({
        "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn teleport() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportAllyToTargetSite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn movement_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fate-threshold-movement" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fate-threshold-movement-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked_site(false),
            "north-fate": fate(),
            "north-heavy": heavy(),
            "north-open": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heavy",
                    "north-fate",
                    "north-heavy",
                    "north-fate",
                    "north-heavy",
                    "north-fate",
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

fn teleport_manifest(seed: u32, ordinary_threshold: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "fate-threshold-teleport",
                "ordinary": ordinary_threshold,
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": if ordinary_threshold {
                "synthetic-fate-threshold-teleport-ordinary-v1"
            } else {
                "synthetic-fate-threshold-teleport-v1"
            },
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked_site(ordinary_threshold),
            "north-fate": fate(),
            "north-heavy": heavy(),
            "north-open": site(),
            "north-teleport": teleport(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heavy",
                    "north-fate",
                    "north-teleport",
                    "north-heavy",
                    "north-fate",
                    "north-teleport",
                    "north-heavy",
                    "north-fate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn movement_opening() -> Session {
    (1..=4096)
        .map(movement_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate threshold movement candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-blocked".to_owned())
                && spells.contains(&"north-heavy".to_owned())
                && spells.contains(&"north-fate".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Fate, a heavy minion, and both sites")
}

fn teleport_opening(ordinary_threshold: bool) -> Session {
    (1..=4096)
        .map(|seed| teleport_manifest(seed, ordinary_threshold))
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate threshold teleport candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-blocked".to_owned())
                && spells.contains(&"north-heavy".to_owned())
                && spells.contains(&"north-fate".to_owned())
                && spells.contains(&"north-teleport".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Fate, Teleport, a heavy minion, and both sites")
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

fn steps_to<'a>(unit_id: &'a str, cell: &'a str) -> impl Fn(&Value) -> bool + 'a {
    move |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == unit_id
            && descriptor["to"]["cell"] == cell
            && descriptor["to"]["region"] == "surface"
    }
}

fn teleports_to<'a>(unit_id: &'a str, cell: &'a str) -> impl Fn(&Value) -> bool + 'a {
    move |descriptor: &Value| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-teleport"
            && descriptor["ally"]["instanceId"] == unit_id
            && descriptor["targetLocation"]["cell"] == cell
    }
}

fn fate_covers(cell: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-fate"
            && descriptor["cells"]
                .as_array()
                .is_some_and(|cells| cells.iter().any(|value| value == cell))
            && descriptor["cells"]
                .as_array()
                .is_some_and(|cells| cells.iter().all(|value| value != "C4"))
    }
}

fn play_named_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn establish_c4_heavy_and_c3_blocked(session: &mut Session) -> String {
    keep(session);
    keep(session);
    play_named_site(session, "north-open", "C4");
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-heavy"
            && descriptor["cell"] == "C4"
    });
    let heavy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("heavy minion identity")
        .to_owned();
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
    play_named_site(session, "north-blocked", "C3");
    heavy_id
}

#[test]
fn rule_catalog_0327_fate_lifts_a_non_ordinary_power_threshold_for_movement() {
    let mut session = movement_opening();
    let heavy_id = establish_c4_heavy_and_c3_blocked(&mut session);
    assert!(!offers(&session, steps_to(&heavy_id, "C3")));
    accept_where(&mut session, fate_covers("C3"));
    assert!(
        offers(&session, steps_to(&heavy_id, "C3")),
        "Fate Lose strips the printed threshold from the uncovered-by-minion site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0328_fate_lifts_teleport_onto_a_non_ordinary_threshold_site() {
    let mut session = teleport_opening(false);
    let heavy_id = establish_c4_heavy_and_c3_blocked(&mut session);
    assert!(!offers(&session, teleports_to(&heavy_id, "C3")));
    accept_where(&mut session, fate_covers("C3"));
    assert!(
        offers(&session, teleports_to(&heavy_id, "C3")),
        "teleport uses the same Lose-aware power-threshold helper as movement"
    );

    let mut ordinary = teleport_opening(true);
    let ordinary_id = establish_c4_heavy_and_c3_blocked(&mut ordinary);
    assert!(!offers(&ordinary, teleports_to(&ordinary_id, "C3")));
    accept_where(&mut ordinary, fate_covers("C3"));
    assert!(
        !offers(&ordinary, teleports_to(&ordinary_id, "C3")),
        "Fate does not strip printed abilities from an Ordinary threshold site"
    );
    assert_exact_replay(&session);
    assert_exact_replay(&ordinary);
}
