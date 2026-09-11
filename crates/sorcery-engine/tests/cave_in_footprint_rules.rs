//! Direct proofs that Cave-In burrows 2×2 occupants of the target land cell
//! (RULE-CATALOG-0353–0354).
//!
//! Cave-In moves every surface minion at the land site underground, then one
//! occupancy settlement kills footprints that cannot exist there. A B3-anchored
//! square occupying C4 is at C4 even when its anchor is not. A mixed land/Water
//! square is still hit; it does not stay on the surface unharmed.

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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn water() -> Value {
    json!({ "cardType": "site", "elements": ["water"] })
}

fn giant() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
        "cardType": "magic",
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
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-footprint-mixed" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-footprint-mixed-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-earth": earth(),
            "north-giant": giant(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant",
                    "north-cave-in",
                    "north-giant",
                    "north-cave-in",
                    "north-giant",
                    "north-cave-in",
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

fn land_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-footprint-land" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-footprint-land-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-earth": earth(),
            "north-giant": giant(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant",
                    "north-cave-in",
                    "north-giant",
                    "north-cave-in",
                    "north-giant",
                    "north-cave-in",
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

fn mixed_opening() -> Session {
    (1..=4096)
        .map(mixed_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Cave-In mixed footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-giant".to_owned())
                && spells.contains(&"north-cave-in".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, Water, a 2x2, and Cave-In")
}

fn land_opening() -> Session {
    (1..=4096)
        .map(land_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Cave-In land footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-giant".to_owned())
                && spells.contains(&"north-cave-in".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, a 2x2, and Cave-In")
}

fn establish_square(session: &mut Session, c3_water: bool) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    if c3_water {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        });
    } else {
        play_site(session, "C3");
    }
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
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

fn cave_in_c4(session: &mut Session) -> Receipt {
    let site_instance = state(session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("C4 site identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_instance
    })
    .1
}

#[test]
fn rule_catalog_0353_cave_in_burrows_mixed_square_occupying_target() {
    let mut session = mixed_opening();
    establish_square(&mut session, true);
    let giant_id = summon_b3_square(&mut session);
    let receipt = cave_in_c4(&mut session);
    assert!(event_types(&receipt).contains(&"minion-burrowed"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-banished")
    );
    let current = state(&session);
    assert!(realm_unit(&current, &giant_id).is_none());
    assert!(cemetery_has(&current, &giant_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0354_cave_in_burrows_all_land_square_occupying_non_anchor() {
    let mut session = land_opening();
    establish_square(&mut session, false);
    let giant_id = summon_b3_square(&mut session);
    let receipt = cave_in_c4(&mut session);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died" || event.event_type == "minion-banished")
    );
    assert!(event_types(&receipt).contains(&"minion-burrowed"));
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id).expect("burrowed 2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(occupant["region"], "underground");
    assert_exact_replay(&session);
}
