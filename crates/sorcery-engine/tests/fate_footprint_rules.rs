//! Direct proofs that Fate Genesis submerges 2×2 occupants of an affected site
//! (RULE-CATALOG-0355–0356).
//!
//! Fate already collects surface occupants by occupancy. It then required the
//! whole footprint to exist underwater before submerging. A B3-anchored square
//! occupying C3 was skipped when the rest of the square was land, so Fate left
//! it on the surface. Submerge every occupant of the affected site; occupancy
//! then kills illegal underwater footprints. An all-Water square with Submerge
//! still survives. Fate must not cover C4 while the Avatar stands there.

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

fn ordinary_earth() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "ordinary": true,
    })
}

fn special_earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn ordinary_water() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
        "ordinary": true,
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

fn giant(submerge: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if submerge {
        value["submerge"] = json!(true);
    }
    value
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

fn mixed_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fate-footprint-mixed" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fate-footprint-mixed-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": ordinary_earth(),
            "north-fate": fate(),
            "north-giant": giant(false),
            "north-special": special_earth(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": ordinary_earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-earth",
                    "north-special",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant",
                    "north-fate",
                    "north-giant",
                    "north-fate",
                    "north-giant",
                    "north-fate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
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

fn water_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fate-footprint-water" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fate-footprint-water-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fate": fate(),
            "north-giant": giant(true),
            "north-special": special_earth(),
            "north-water": ordinary_water(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": ordinary_earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-special",
                    "north-water",
                    "north-water",
                    "north-water",
                    "north-water",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant",
                    "north-fate",
                    "north-giant",
                    "north-fate",
                    "north-giant",
                    "north-fate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
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
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn cemetery_has(current: &Value, instance_id: &str) -> bool {
    ["north", "south"].into_iter().any(|seat| {
        current["players"][seat]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id)
    })
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

fn mixed_opening() -> Session {
    (1..=4096)
        .map(mixed_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate mixed footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && atlas.contains(&"north-special".to_owned())
                && spells.contains(&"north-giant".to_owned())
                && spells.contains(&"north-fate".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with ordinary earth, a non-Ordinary site, a 2x2, and Fate")
}

fn water_opening() -> Session {
    (1..=4096)
        .map(water_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate water footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && atlas.contains(&"north-special".to_owned())
                && spells.contains(&"north-giant".to_owned())
                && spells.contains(&"north-fate".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Water, a non-Ordinary site, a 2x2, and Fate")
}

fn establish_square(session: &mut Session, first_and_edges: &str, c3_card: &str) {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == first_and_edges
            && descriptor["cell"] == "C4"
    });
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == first_and_edges
            && descriptor["cell"] == "B4"
    });
    end_and_draw(session);
    end_and_draw(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == c3_card
            && descriptor["cell"] == "C3"
    });
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == first_and_edges
            && descriptor["cell"] == "B3"
    });
}

fn summon_b3_square(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 identity")
        .to_owned();
    let current = state(session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(occupant["region"], "surface");
    giant_id
}

fn cast_fate_on_c3(session: &mut Session) -> Receipt {
    accept_where(session, fate_covers_c3_not_c4).1
}

#[test]
fn rule_catalog_0355_fate_submerges_mixed_square_occupying_affected_site() {
    let mut session = mixed_opening();
    establish_square(&mut session, "north-earth", "north-special");
    let giant_id = summon_b3_square(&mut session);
    let receipt = cast_fate_on_c3(&mut session);
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert!(event_types(&receipt).contains(&"minion-submerged"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    let current = state(&session);
    assert!(realm_unit(&current, &giant_id).is_none());
    assert!(cemetery_has(&current, &giant_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0356_fate_submerges_all_water_square_occupying_affected_site() {
    let mut session = water_opening();
    establish_square(&mut session, "north-water", "north-special");
    let giant_id = summon_b3_square(&mut session);
    let receipt = cast_fate_on_c3(&mut session);
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert!(event_types(&receipt).contains(&"minion-submerged"));
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died" || event.event_type == "minion-banished")
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id).expect("submerged 2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &giant_id));
    assert_exact_replay(&session);
}
