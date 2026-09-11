//! Direct proofs that returning a site banishes lower-layer occupants
//! (RULE-CATALOG-0345–0346).
//!
//! Return-site already remaps surface occupants into the void so they banish
//! instead of dying as if stranded. The site leaves play with no rubble, so
//! underground and underwater locations are gone too. Those occupants use the
//! same void remap.

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

fn dualer() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bounce() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn earth_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-site-layers-earth" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-layers-earth-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "south-avatar": avatar(),
            "south-bounce": bounce(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-dualer"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-bounce"; 6],
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
            "contentHash": identity_hash(&json!({ "fixture": "return-site-layers-water" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-layers-water-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-bounce": bounce(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-dualer"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-bounce"; 6],
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

fn opening_ids(session: &Session, seat: &str, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"][seat]["hand"][zone]
        .as_array()
        .expect("hand zone")
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

fn cemetery_has(current: &Value, instance_id: &str) -> bool {
    ["north", "south"].into_iter().any(|seat| {
        current["players"][seat]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id)
    })
}

fn realm_has_unit(current: &Value, instance_id: &str) -> bool {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .any(|unit| unit["instanceId"] == instance_id)
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

fn earth_opening() -> Session {
    (1..=4096)
        .map(earth_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("return-site earth candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-bounce".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, a dual-region minion, and return-site")
}

fn water_opening() -> Session {
    (1..=4096)
        .map(water_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("return-site Water candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-bounce".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Water, a dual-region minion, and return-site")
}

fn summon_on_c3_then_south_ready(session: &mut Session, site_id: &str, region: &str) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C4"
    });
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
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C3"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == region
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    dualer_id
}

fn return_c3(session: &mut Session) -> Receipt {
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-bounce"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    receipt
}

fn assert_banished_not_killed(session: &Session, receipt: &Receipt, dualer_id: &str) {
    assert_eq!(
        event_types(receipt),
        [
            "magic-cast",
            "site-returned-to-hand",
            "minion-banished",
            "magic-resolved"
        ]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died"
                || event.event_type == "rubble-created"
                || event.event_type == "site-destroyed")
    );
    let current = state(session);
    assert!(current["realm"]["sites"].get("C3").is_none());
    assert!(!realm_has_unit(&current, dualer_id));
    assert!(!cemetery_has(&current, dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_0345_returning_land_banishes_underground_minion() {
    let mut session = earth_opening();
    let dualer_id = summon_on_c3_then_south_ready(&mut session, "north-earth", "underground");
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == dualer_id)
            .expect("underground dual-region unit")["region"],
        "underground"
    );
    let receipt = return_c3(&mut session);
    assert_banished_not_killed(&session, &receipt, &dualer_id);
}

#[test]
fn rule_catalog_0346_returning_water_banishes_underwater_minion() {
    let mut session = water_opening();
    let dualer_id = summon_on_c3_then_south_ready(&mut session, "north-water", "underwater");
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == dualer_id)
            .expect("underwater dual-region unit")["region"],
        "underwater"
    );
    let receipt = return_c3(&mut session);
    assert_banished_not_killed(&session, &receipt, &dualer_id);
}
