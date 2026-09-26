//! Direct proofs that leaving Flood or Drought relayers lower-layer occupants
//! (RULE-CATALOG-0339–0340, RULE-CATALOG-1336–1337, RULE-CATALOG-1355–1358,
//! RULE-CATALOG-1363–1366, RULE-CATALOG-1415–1418, RULE-CATALOG-1425–1428,
//! RULE-CATALOG-1481–1483, RULE-CATALOG-1505–1508),
//! and that destroy-target-
//! aura or return-target-aura Magic on Flood or Drought stays withheld during
//! trigger-order (RULE-CATALOG-1331–1334, RULE-CATALOG-1485–1486,
//! RULE-CATALOG-1494–1496, RULE-CATALOG-1503–1504, RULE-CATALOG-1519,
//! RULE-CATALOG-1525–1528).
//!
//! Overlay Auras already convert underground and underwater when they enter.
//! Destroying or returning that Aura flips the site's water-ness again, so the
//! same conversion runs in reverse: a Burrowing and Submerge unit Flood saved
//! underwater returns underground, and one Drought moved underground returns
//! underwater.

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

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drought() -> Value {
    json!({
        "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn destroy_aura() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetAura": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn return_aura() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetAuraToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn flood_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-flood" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-destroy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-destroy"; 6],
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

fn covers_c3(descriptor: &Value, card_id: &str) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == card_id
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C3"))
}

fn covers_c4(descriptor: &Value, card_id: &str) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == card_id
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C4"))
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> &'a Value {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("dual-region unit remains in play")
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

fn flood_opening() -> Session {
    (1..=4096)
        .map(flood_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood leave candidate");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-destroy".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, a dual-region minion, and destroy-Aura")
}

fn drought_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought leave candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-destroy".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Water, a dual-region minion, and destroy-Aura")
}

fn flood_water_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-flood-water" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-flood-water-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-flood": flood(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-destroy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_earth_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-drought-earth" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-drought-earth-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-destroy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn flood_water_opening() -> Session {
    (1..=4096)
        .map(flood_water_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood water leave candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-destroy".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, Water, a dual-region minion, and destroy-Aura")
}

fn drought_earth_opening() -> Session {
    (1..=4096)
        .map(drought_earth_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought earth leave candidate");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-destroy".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Earth, a dual-region minion, and destroy-Aura")
}

fn flood_return_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-flood-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-flood-return-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "south-avatar": avatar(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-return"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_return_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-drought-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-drought-return-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-return"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn flood_return_opening() -> Session {
    (1..=4096)
        .map(flood_return_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood return leave candidate");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-return".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, a dual-region minion, and return-Aura")
}

fn drought_return_opening() -> Session {
    (1..=4096)
        .map(drought_return_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought return leave candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-return".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Water, a dual-region minion, and return-Aura")
}

fn flood_water_return_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-flood-water-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-flood-water-return-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-flood": flood(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                    "north-flood",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-return"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_earth_return_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-leave-drought-earth-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-leave-drought-earth-return-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "south-avatar": avatar(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                    "north-drought",
                    "north-dualer",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-return"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn flood_water_return_opening() -> Session {
    (1..=4096)
        .map(flood_water_return_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood water return leave candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-return".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, Water, a dual-region minion, and return-Aura")
}

fn drought_earth_return_opening() -> Session {
    (1..=4096)
        .map(drought_earth_return_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought earth return leave candidate");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && south.contains(&"south-return".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Earth, a dual-region minion, and return-Aura")
}

fn play_overlay_then_south_ready(
    session: &mut Session,
    site_id: &str,
    region: &str,
    aura_id: &str,
) -> (String, String) {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == region
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    accept_where(session, |descriptor| covers_c4(descriptor, aura_id));
    let aura_instance = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("overlay identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    (dualer_id, aura_instance)
}

fn play_overlay_on_occupied_c3(
    session: &mut Session,
    site_id: &str,
    region: &str,
    aura_id: &str,
) -> (String, String) {
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
    accept_where(session, |descriptor| covers_c3(descriptor, aura_id));
    let aura_instance = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("overlay identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (dualer_id, aura_instance)
}

#[test]
fn rule_catalog_0339_destroying_flood_relayers_dual_region_minion_underground() {
    let mut session = flood_opening();
    let (dualer_id, aura_id) =
        play_overlay_then_south_ready(&mut session, "north-earth", "underground", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "leaving Flood must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C4");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0340_destroying_drought_relayers_dual_region_minion_underwater() {
    let mut session = drought_opening();
    let (dualer_id, aura_id) =
        play_overlay_then_south_ready(&mut session, "north-water", "underwater", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "leaving Drought must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C4");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn flood_destroy_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-destroy-flood-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-destroy-flood-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-rain": rain_spell(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn flood_water_occupied_destroy_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-destroy-flood-water-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-destroy-flood-water-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-flood": flood(),
            "north-rain": rain_spell(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_earth_occupied_destroy_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-destroy-drought-earth-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-destroy-drought-earth-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-rain": rain_spell(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_destroy_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-destroy-drought-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-destroy-drought-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-rain": rain_spell(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_aura(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                    "south-minion",
                    "south-destroy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PendingOverlayDestroySetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    dualer_id: String,
    session: Session,
}

fn try_accept_where_overlay(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn offers_destroy_aura(session: &Session, aura_id: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "south-destroy"
                && action.descriptor["targetAuraInstanceId"] == aura_id
        })
    })
}

fn try_pending_deathrite_with_overlay_destroy(
    encoded: &str,
    site_id: &str,
    region: &str,
    overlay_card: &str,
) -> Option<PendingOverlayDestroySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C4"
    })?;
    let (summoned, _) = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == region
    })?;
    let dualer_id = summoned["cardInstanceId"].as_str()?.to_owned();
    try_accept_where_overlay(&mut session, |descriptor| {
        covers_c4(descriptor, overlay_card)
    })?;
    let overlay_id = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["cardId"] == "north-rain")
    {
        return None;
    }
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingOverlayDestroySetup {
        aura_id: overlay_id,
        deathrite_ids,
        dualer_id,
        session,
    })
}

fn try_pending_deathrite_with_overlay_occupied_destroy(
    encoded: &str,
    site_id: &str,
    region: &str,
    overlay_card: &str,
) -> Option<PendingOverlayDestroySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == site_id
            && descriptor["cell"] == "C3"
    })?;
    let (summoned, _) = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == region
    })?;
    let dualer_id = summoned["cardInstanceId"].as_str()?.to_owned();
    try_accept_where_overlay(&mut session, |descriptor| {
        covers_c3(descriptor, overlay_card)
    })?;
    let overlay_id = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where_overlay(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["cardId"] == "north-rain")
    {
        return None;
    }
    try_accept_where_overlay(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingOverlayDestroySetup {
        aura_id: overlay_id,
        deathrite_ids,
        dualer_id,
        session,
    })
}

fn try_pending_deathrite_with_overlay_occupied_return(
    encoded: &str,
    site_id: &str,
    region: &str,
    overlay_card: &str,
) -> Option<PendingOverlayDestroySetup> {
    let setup = try_pending_deathrite_with_overlay_occupied_destroy(
        encoded,
        site_id,
        region,
        overlay_card,
    )?;
    if offers_return_aura(&setup.session, &setup.aura_id) {
        return None;
    }
    Some(setup)
}

fn flood_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_destroy(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect("bounded seed that reaches pending Deathrites with Flood destroy target")
}

fn drought_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_destroy(
                candidate,
                "north-water",
                "underwater",
                "north-drought",
            )
            .is_some()
        })
        .expect("bounded seed that reaches pending Deathrites with Drought destroy target")
}

fn flood_occupied_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood destroy on occupied Earth site",
        )
}

fn flood_occupied_earth_c3_destroy_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flood_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood destroy on occupied Earth at C3",
        )
}

fn flood_occupied_earth_c3_return_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flood_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood return on occupied Earth at C3",
        )
}

fn drought_occupied_water_c3_return_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-water",
                "underwater",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought return on occupied Water at C3",
        )
}

fn flood_occupied_water_c3_return_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flood_water_occupied_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-water",
                "underwater",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood return on flooded occupied Water at C3",
        )
}

fn drought_occupied_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-water",
                "underwater",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought destroy on occupied Water site",
        )
}

fn flood_occupied_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood return on occupied Earth site",
        )
}

fn drought_occupied_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-water",
                "underwater",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought return on occupied Water site",
        )
}

fn flood_water_occupied_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_water_occupied_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-water",
                "underwater",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood destroy on occupied Water site",
        )
}

fn flood_occupied_water_c3_destroy_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flood_water_occupied_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-water",
                "underwater",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood destroy on flooded occupied Water at C3",
        )
}

fn drought_earth_occupied_destroy_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_earth_occupied_destroy_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_destroy(
                candidate,
                "north-earth",
                "underground",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought destroy on occupied Earth site",
        )
}

fn flood_water_occupied_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_water_occupied_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-water",
                "underwater",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood return on occupied Water site",
        )
}

fn flood_water_occupied_c3_return_withheld_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flood_water_occupied_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-water",
                "underwater",
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood return on flooded occupied Water at C3",
        )
}

fn drought_earth_occupied_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_earth_occupied_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-earth",
                "underground",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought return on occupied Earth site",
        )
}

fn drought_occupied_earth_c3_return_relayer_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_occupied_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_return(
                candidate,
                "north-earth",
                "underground",
                "north-drought",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought return on drought occupied Earth at C3",
        )
}

#[test]
fn rule_catalog_1331_destroy_flood_aura_withheld_during_pending_deathrite_order() {
    let encoded = flood_destroy_withheld_seed_with(1331);
    let mut setup = try_pending_deathrite_with_overlay_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood destroy Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1332_destroy_drought_aura_withheld_during_pending_deathrite_order() {
    let encoded = drought_destroy_withheld_seed_with(1332);
    let mut setup = try_pending_deathrite_with_overlay_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought destroy Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1336_returning_flood_relayers_dual_region_minion_underground() {
    let mut session = flood_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_then_south_ready(&mut session, "north-earth", "underground", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Flood must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C4");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1337_returning_drought_relayers_dual_region_minion_underwater() {
    let mut session = drought_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_then_south_ready(&mut session, "north-water", "underwater", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Drought must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C4");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

fn flood_return_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-return-flood-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-return-flood-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-rain": rain_spell(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn flood_water_occupied_return_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-return-flood-water-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-return-flood-water-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-flood": flood(),
            "north-rain": rain_spell(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                    "north-flood",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_earth_occupied_return_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-return-drought-earth-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-return-drought-earth-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-rain": rain_spell(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_return_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-return-drought-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-return-drought-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-rain": rain_spell(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-return": return_aura(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                    "north-drought",
                    "north-dualer",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                    "south-minion",
                    "south-return",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn offers_return_aura(session: &Session, aura_id: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "south-return"
                && action.descriptor["targetAuraInstanceId"] == aura_id
        })
    })
}

fn try_pending_deathrite_with_overlay_return(
    encoded: &str,
    site_id: &str,
    region: &str,
    overlay_card: &str,
) -> Option<PendingOverlayDestroySetup> {
    let setup = try_pending_deathrite_with_overlay_destroy(encoded, site_id, region, overlay_card)?;
    if offers_return_aura(&setup.session, &setup.aura_id) {
        return None;
    }
    Some(setup)
}

fn flood_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flood_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_return(
                candidate,
                "north-earth",
                "underground",
                "north-flood",
            )
            .is_some()
        })
        .expect("bounded seed that reaches pending Deathrites with Flood return target")
}

fn drought_return_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_return_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_return(
                candidate,
                "north-water",
                "underwater",
                "north-drought",
            )
            .is_some()
        })
        .expect("bounded seed that reaches pending Deathrites with Drought return target")
}

#[test]
fn rule_catalog_1333_return_flood_aura_withheld_during_pending_deathrite_order() {
    let encoded = flood_return_withheld_seed_with(1333);
    let mut setup = try_pending_deathrite_with_overlay_return(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood return Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1334_return_drought_aura_withheld_during_pending_deathrite_order() {
    let encoded = drought_return_withheld_seed_with(1334);
    let mut setup = try_pending_deathrite_with_overlay_return(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought return Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1355_destroying_flood_on_occupied_earth_site_relayers_dual_region_minion_underground()
 {
    let mut session = flood_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-earth", "underground", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Flood on occupied Earth must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1356_destroying_drought_on_occupied_water_site_relayers_dual_region_minion_underwater()
 {
    let mut session = drought_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-water", "underwater", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Drought on occupied Water must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1357_returning_flood_on_occupied_earth_site_relayers_dual_region_minion_underground()
 {
    let mut session = flood_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-earth", "underground", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Flood on occupied Earth must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1358_returning_drought_on_occupied_water_site_relayers_dual_region_minion_underwater()
 {
    let mut session = drought_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-water", "underwater", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Drought on occupied Water must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1363_destroy_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = flood_occupied_destroy_withheld_seed_with(1363);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood destroy on occupied Earth site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1364_destroy_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = drought_occupied_destroy_withheld_seed_with(1364);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought destroy on occupied Water site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1365_return_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = flood_occupied_return_withheld_seed_with(1365);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood return on occupied Earth site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1366_return_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = drought_occupied_return_withheld_seed_with(1366);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought return on occupied Water site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1415_destroy_flood_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = flood_water_occupied_destroy_withheld_seed_with(1415);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood destroy on occupied Water site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1416_destroy_drought_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = drought_earth_occupied_destroy_withheld_seed_with(1416);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect("complete Drought destroy on occupied Earth site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1417_return_flood_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = flood_water_occupied_return_withheld_seed_with(1417);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood return on occupied Water site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1418_return_drought_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = drought_earth_occupied_return_withheld_seed_with(1418);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect("complete Drought return on occupied Earth site Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1425_destroying_flood_on_occupied_water_site_relayers_dual_region_minion_underwater()
 {
    let mut session = flood_water_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-water", "underwater", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Flood on occupied Water must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1426_destroying_drought_on_occupied_earth_site_relayers_dual_region_minion_underground()
 {
    let mut session = drought_earth_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-earth", "underground", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Drought on occupied Earth must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1427_returning_flood_on_occupied_water_site_relayers_dual_region_minion_underwater()
{
    let mut session = flood_water_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-water", "underwater", "north-flood");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Flood on occupied Water must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1428_returning_drought_on_occupied_earth_site_relayers_dual_region_minion_underground()
 {
    let mut session = drought_earth_return_opening();
    let (dualer_id, aura_id) =
        play_overlay_on_occupied_c3(&mut session, "north-earth", "underground", "north-drought");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Drought on occupied Earth must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1481_destroying_flood_on_occupied_earth_at_c3_relayers_underwater_after_deathrite_order()
 {
    let encoded = flood_occupied_earth_c3_destroy_relayer_seed_with(1481);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood destroy on occupied Earth site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underwater"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Flood on occupied Earth after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1482_destroying_drought_on_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let encoded = drought_occupied_destroy_withheld_seed_with(1482);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought destroy on occupied Water site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underground"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Drought on occupied Water after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1486_return_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site_at_c3()
 {
    let encoded = drought_occupied_return_withheld_seed_with(1486);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought return on occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1485_return_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site_at_c3()
 {
    let encoded = flood_occupied_return_withheld_seed_with(1485);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood return on occupied Earth site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1483_returning_flood_on_occupied_earth_at_c3_relayers_underground_after_deathrite_order()
 {
    let encoded = flood_occupied_earth_c3_return_relayer_seed_with(1483);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood return on occupied Earth site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underwater"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Flood on occupied Earth after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1484_returning_drought_on_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let encoded = drought_occupied_water_c3_return_relayer_seed_with(1484);
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought return on occupied Water site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underground"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Drought on occupied Water after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1505_returning_flood_on_flooded_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let encoded = flood_occupied_water_c3_return_relayer_seed_with(1505);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood return on flooded occupied Water site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underwater"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Flood on flooded occupied Water after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1493_destroy_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site_at_c3()
 {
    let encoded = flood_occupied_earth_c3_destroy_relayer_seed_with(1493);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood destroy on occupied Earth site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1494_destroy_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site_at_c3()
 {
    let encoded = drought_occupied_destroy_withheld_seed_with(1494);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-drought",
    )
    .expect("complete Drought destroy on occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1495_destroy_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_water_site_at_c3()
 {
    let encoded = flood_water_occupied_destroy_withheld_seed_with(1495);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood destroy on flooded occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1496_destroy_drought_aura_withheld_during_pending_deathrite_order_on_drought_occupied_earth_site_at_c3()
 {
    let encoded = drought_earth_occupied_destroy_withheld_seed_with(1496);
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect(
        "complete Drought destroy on drought occupied Earth site at C3 Deathrite withheld setup",
    );
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1503_return_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_water_site_at_c3()
 {
    let encoded = flood_water_occupied_c3_return_withheld_seed_with(1503);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood return on flooded occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1504_return_drought_aura_withheld_during_pending_deathrite_order_on_drought_occupied_earth_site_at_c3()
 {
    let encoded = drought_earth_occupied_return_withheld_seed_with(1504);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect(
        "complete Drought return on drought occupied Earth site at C3 Deathrite withheld setup",
    );
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1506_returning_drought_on_drought_occupied_earth_at_c3_relayers_underground_after_deathrite_order()
 {
    let encoded = drought_occupied_earth_c3_return_relayer_seed_with(1506);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect("complete Drought return on drought occupied Earth site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underground"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_return_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_return_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "returning Drought on drought occupied Earth after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1507_destroying_flood_on_flooded_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let encoded = flood_occupied_water_c3_destroy_relayer_seed_with(1507);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded relayer manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood destroy on flooded occupied Water site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underwater"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Flood on flooded occupied Water after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1508_destroying_drought_on_drought_occupied_earth_at_c3_relayers_underground_after_deathrite_order()
 {
    let encoded = drought_earth_occupied_destroy_withheld_seed_with(1508);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect("complete Drought destroy on drought occupied Earth site Deathrite relayer setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underground"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_destroy_aura(session, &aura_id));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    if state(session)["decisionSeat"] == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    assert_eq!(state(session)["decisionSeat"], "south");
    assert!(offers_destroy_aura(session, &aura_id));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "destroying Drought on drought occupied Earth after trigger-order must relayer the dual-region unit instead of killing it"
    );
    let current = state(session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1519_return_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_water_site_at_c3()
 {
    let encoded = flood_water_occupied_c3_return_withheld_seed_with(1519);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood return on flooded occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(
        !offers_return_aura(&setup.session, &aura_id),
        "return-target-aura Magic on Flood stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1525_return_drought_aura_withheld_during_pending_deathrite_order_on_drought_occupied_earth_site_at_c3()
 {
    let encoded = drought_earth_occupied_return_withheld_seed_with(1525);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect(
        "complete Drought return on drought occupied Earth site at C3 Deathrite withheld setup",
    );
    let aura_id = setup.aura_id.clone();
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(
        !offers_return_aura(&setup.session, &aura_id),
        "return-target-aura Magic on Drought stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1526_destroy_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_water_site_at_c3()
 {
    let encoded = flood_water_occupied_destroy_withheld_seed_with(1526);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-water",
        "underwater",
        "north-flood",
    )
    .expect("complete Flood destroy on flooded occupied Water site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(
        !offers_destroy_aura(&setup.session, &aura_id),
        "destroy-target-aura Magic on Flood stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1527_destroy_drought_aura_withheld_during_pending_deathrite_order_on_drought_occupied_earth_site_at_c3()
 {
    let encoded = drought_earth_occupied_destroy_withheld_seed_with(1527);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let setup = try_pending_deathrite_with_overlay_occupied_destroy(
        &encoded,
        "north-earth",
        "underground",
        "north-drought",
    )
    .expect(
        "complete Drought destroy on drought occupied Earth site at C3 Deathrite withheld setup",
    );
    let aura_id = setup.aura_id.clone();
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(
        !offers_destroy_aura(&setup.session, &aura_id),
        "destroy-target-aura Magic on Drought stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1528_return_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_earth_site_at_c3()
 {
    let encoded = flood_occupied_return_withheld_seed_with(1528);
    let seed = serde_json::from_str::<Value>(&encoded).expect("encoded withheld manifest")["seed"]
        .as_u64()
        .expect("numeric seed");
    eprintln!("seed={seed}");
    let setup = try_pending_deathrite_with_overlay_occupied_return(
        &encoded,
        "north-earth",
        "underground",
        "north-flood",
    )
    .expect("complete Flood return on flooded occupied Earth site at C3 Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(
        !offers_return_aura(&setup.session, &aura_id),
        "return-target-aura Magic on Flood stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}
