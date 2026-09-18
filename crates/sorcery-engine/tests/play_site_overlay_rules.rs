//! Direct proofs that playing a site onto overlay-covered rubble relayers
//! lower-layer occupants (RULE-CATALOG-0343–0344), and that Flood/Drought
//! conversion creates the expected effective site type (RULE-CATALOG-1319,
//! RULE-CATALOG-1321, RULE-CATALOG-1323, RULE-CATALOG-1325), and same-type
//! overlay play relayers lower-layer occupants (RULE-CATALOG-1328,
//! RULE-CATALOG-1329), overlay conversion on occupied sites plus
//! replacement play after destroy (RULE-CATALOG-1338–1339,
//! RULE-CATALOG-1344–1345, RULE-CATALOG-1349–1350, RULE-CATALOG-1367–1368,
//! RULE-CATALOG-1377–1381, RULE-CATALOG-1387–1390, RULE-CATALOG-1392,
//! RULE-CATALOG-1397–1398, RULE-CATALOG-1401–1402, RULE-CATALOG-1412–1413,
//! RULE-CATALOG-1471–1474, RULE-CATALOG-1513–1514).
//!
//! Playing Water onto rubble already floods underground occupants. Overlay
//! Auras already convert layers when they enter or leave. Playing a site onto
//! rubble still covered by Flood or Drought must use that same conversion:
//! printed earth under Flood becomes Water, and printed Water under Drought
//! becomes land.

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

fn destroy_site() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
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

fn water_cast() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "mustBeCastToWaterSite": true,
        "summonToAnySite": true,
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
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-water",
                ],
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
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-water",
                ],
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

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
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

fn summon_cells(session: &Session, card_id: &str) -> Vec<String> {
    let mut cells = session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion" && action.descriptor["cardId"] == card_id
        })
        .filter_map(|action| action.descriptor["cell"].as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    cells.sort();
    cells.dedup();
    cells
}

fn flood_site_type_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood-site-type" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-site-type-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-water": water(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
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
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-water-cast",
                    "north-flood",
                    "north-water-cast",
                    "north-flood",
                    "north-water-cast",
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

fn drought_site_type_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-drought-site-type" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-drought-site-type-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-earth": earth(),
            "north-water": water(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
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

fn flood_site_type_opening() -> Session {
    (1..=4096)
        .map(flood_site_type_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood site-type candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().any(|card| card == "north-earth")
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, earth site, and water-site cast")
}

fn drought_site_type_opening() -> Session {
    (1..=4096)
        .map(drought_site_type_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought site-type candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().any(|card| card == "north-water")
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Water site, and water-site cast")
}

fn flood_water_site_type_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood-water-site-type" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-water-site-type-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-water": water(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-water-cast",
                    "north-flood",
                    "north-water-cast",
                    "north-flood",
                    "north-water-cast",
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

fn drought_earth_site_type_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-drought-earth-site-type" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-drought-earth-site-type-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-earth": earth(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
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

fn flood_water_site_type_opening() -> Session {
    (1..=4096)
        .map(flood_water_site_type_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood water site-type candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().any(|card| card == "north-water")
                && atlas.iter().any(|card| card == "north-earth")
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, Water site, Earth site, and water-site cast")
}

fn drought_earth_site_type_opening() -> Session {
    (1..=4096)
        .map(drought_earth_site_type_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought earth site-type candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().any(|card| card == "north-earth")
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Earth site, and water-site cast")
}

fn rubble_at_c3(session: &mut Session, setup_site_id: &str) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
            && descriptor["cardId"] == setup_site_id
            && descriptor["cell"] == "C3"
    });
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    assert_eq!(state(session)["realm"]["sites"]["C3"]["rubble"], true);
    site_instance
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
            let session = Session::new(&candidate).expect("play-site Flood candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, a dual-region minion, and three earth sites")
}

fn drought_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("play-site Drought candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-earth".to_owned())
                && atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect(
            "bounded seed opening with Drought, a dual-region minion, earth, and two Water sites",
        )
}

fn bury_then_cover_c3(
    session: &mut Session,
    setup_site_id: &str,
    site_id: &str,
    region: &str,
    aura_id: &str,
) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == setup_site_id
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
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        "underground"
    );
    accept_where(session, |descriptor| covers_c3(descriptor, aura_id));
    dualer_id
}

#[test]
fn rule_catalog_0343_playing_earth_onto_flooded_rubble_relayers_underwater() {
    let mut session = flood_opening();
    let dualer_id = bury_then_cover_c3(
        &mut session,
        "north-earth",
        "north-earth",
        "underground",
        "north-flood",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing earth onto Flood-covered rubble must relayer the dual-region unit"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0344_playing_water_onto_drought_rubble_relayers_underground() {
    let mut session = drought_opening();
    let dualer_id = bury_then_cover_c3(
        &mut session,
        "north-earth",
        "north-water",
        "underwater",
        "north-drought",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto Drought-covered rubble must relayer the dual-region unit"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1319_playing_earth_onto_flooded_rubble_creates_water_site() {
    let mut session = flooded_earth_site_rubble_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed earth played onto Flood-covered rubble must become a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1321_playing_water_onto_drought_rubble_creates_earth_site() {
    let mut session = drought_site_type_opening();
    rubble_at_c3(&mut session, "north-water");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        covers_c3(descriptor, "north-drought")
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto Drought-covered rubble must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1323_playing_water_onto_flooded_rubble_creates_water_site() {
    let mut session = flood_water_site_type_opening();
    rubble_at_c3(&mut session, "north-water");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        covers_c3(descriptor, "north-flood")
    });
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "empty flooded rubble must not accept a water-site cast before site play"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto Flood-covered rubble must remain a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1325_playing_earth_onto_drought_rubble_creates_earth_site() {
    let mut session = drought_earth_site_type_opening();
    rubble_at_c3(&mut session, "north-earth");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        covers_c3(descriptor, "north-drought")
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto Drought-covered rubble must remain land"
    );
    assert_exact_replay(&session);
}

fn flood_opening_with_water_site() -> Session {
    (1..=4096)
        .map(flood_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Flood water-site candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect(
            "bounded seed opening with Flood, Water site in hand, earth sites, and dual-region minion",
        )
}

fn drought_earth_play_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought earth-play candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-earth".to_owned())
                && atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect(
            "bounded seed opening with Drought, Earth site in hand, Water sites, and dual-region minion",
        )
}

#[test]
fn rule_catalog_1328_playing_water_onto_flooded_rubble_relayers_underwater() {
    let mut session = flood_opening_with_water_site();
    let dualer_id = bury_then_cover_c3(
        &mut session,
        "north-earth",
        "north-earth",
        "underground",
        "north-flood",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto Flood-covered rubble must relayer the dual-region unit"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1329_playing_earth_onto_drought_rubble_relayers_underground() {
    let mut session = drought_earth_play_opening();
    let dualer_id = bury_then_cover_c3(
        &mut session,
        "north-water",
        "north-water",
        "underwater",
        "north-drought",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto Drought-covered rubble must relayer the dual-region unit"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

fn occupied_site_at_c3(session: &mut Session, setup_site_id: &str) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
            && descriptor["cardId"] == setup_site_id
            && descriptor["cell"] == "C3"
    });
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        setup_site_id
    );
    assert_ne!(state(session)["realm"]["sites"]["C3"]["rubble"], true);
    site_instance
}

fn pass_turn_and_draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn offers_play_earth_on_c3(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    })
}

fn offers_play_water_on_c3(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    })
}

fn maybe_draw_site_for_play(session: &mut Session) {
    if session.legal_actions().is_ok_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == "draw-site")
    }) {
        accept_where(session, |descriptor| descriptor["kind"] == "draw-site");
    }
}

fn drought_occupied_water_rubble_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&drought_site_type_manifest(seed)).ok()?;
            let site_instance = drought_water_site_at_c3(&mut session);
            destroy_occupied_site_at_c3(&mut session, &site_instance);
            pass_turn_and_draw_spellbook(&mut session);
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            if session.legal_actions().is_ok_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.descriptor["kind"] == "draw-site")
            }) {
                accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
            }
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after drought occupied-water rubble")
}

fn flooded_earth_site_rubble_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&flood_site_type_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            {
                return None;
            }
            let site_instance = flooded_earth_site_at_c3(&mut session);
            destroy_occupied_site_at_c3(&mut session, &site_instance);
            if summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()) {
                return None;
            }
            pass_turn_and_draw_spellbook(&mut session);
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after flooded occupied-earth rubble")
}

fn drought_occupied_water_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&drought_site_type_manifest(seed)).ok()?;
            drought_water_site_at_c3(&mut session);
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            if session.legal_actions().is_ok_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.descriptor["kind"] == "draw-site")
            }) {
                accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
            }
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after drought occupied Water site")
}

fn flood_occupied_earth_site_type_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood-occupied-earth-site-type" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-occupied-earth-site-type-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
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
                    "north-water-cast",
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

fn flood_occupied_earth_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&flood_occupied_earth_site_type_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            {
                return None;
            }
            cover_occupied_dualer_at_c3(
                &mut session,
                "north-earth",
                "north-earth",
                "underground",
                "north-flood",
                "underwater",
            );
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            if session.legal_actions().is_ok_and(|actions| {
                actions
                    .iter()
                    .any(|action| action.descriptor["kind"] == "draw-site")
            }) {
                accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
            }
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after flooded occupied Earth site")
}

fn flooded_earth_site_at_c3(session: &mut Session) -> String {
    let site_instance = occupied_site_at_c3(session, "north-earth");
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, "north-flood"));
    site_instance
}

fn drought_water_site_at_c3(session: &mut Session) -> String {
    let site_instance = occupied_site_at_c3(session, "north-water");
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, "north-drought"));
    site_instance
}

fn flooded_water_site_at_c3(session: &mut Session) -> String {
    let site_instance = occupied_site_at_c3(session, "north-water");
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, "north-flood"));
    site_instance
}

fn drought_earth_site_at_c3(session: &mut Session) -> String {
    let site_instance = occupied_site_at_c3(session, "north-earth");
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, "north-drought"));
    site_instance
}

fn destroy_occupied_site_at_c3(session: &mut Session, site_instance: &str) {
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    assert_eq!(state(session)["realm"]["sites"]["C3"]["rubble"], true);
}

fn cover_occupied_dualer_at_c3(
    session: &mut Session,
    setup_site_id: &str,
    site_id: &str,
    summon_region: &str,
    aura_id: &str,
    relayer_region: &str,
) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == setup_site_id
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
            && descriptor["region"] == summon_region
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, aura_id));
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        relayer_region
    );
    dualer_id
}

fn cover_occupied_dualer_then_destroy_c3(
    session: &mut Session,
    setup_site_id: &str,
    site_id: &str,
    summon_region: &str,
    aura_id: &str,
    relayer_region: &str,
) -> String {
    let dualer_id = cover_occupied_dualer_at_c3(
        session,
        setup_site_id,
        site_id,
        summon_region,
        aura_id,
        relayer_region,
    );
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-destroy"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    pass_turn_and_draw_spellbook(session);
    assert_eq!(state(session)["realm"]["sites"]["C3"]["rubble"], true);
    dualer_id
}

#[test]
fn rule_catalog_1338_playing_earth_onto_flooded_earth_site_creates_water_site() {
    let mut session = flooded_earth_site_rubble_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto Flood-covered Rubble from a flooded Earth site must become a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1339_playing_water_onto_drought_water_site_creates_land() {
    let mut session = drought_site_type_opening();
    let site_instance = drought_water_site_at_c3(&mut session);
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Drought on an occupied Water site must make that cell count as land immediately"
    );
    destroy_occupied_site_at_c3(&mut session, &site_instance);
    pass_turn_and_draw_spellbook(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto Drought-covered Rubble from a drought Water site must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1344_playing_water_onto_flooded_occupied_earth_rubble_relayers_underwater() {
    let mut session = flood_opening_with_water_site();
    let dualer_id = cover_occupied_dualer_then_destroy_c3(
        &mut session,
        "north-earth",
        "north-earth",
        "underground",
        "north-flood",
        "underwater",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto Flood-covered Rubble from a flooded occupied Earth site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1345_playing_earth_onto_drought_occupied_water_rubble_relayers_underground() {
    let mut session = drought_earth_play_opening();
    let dualer_id = cover_occupied_dualer_then_destroy_c3(
        &mut session,
        "north-water",
        "north-water",
        "underwater",
        "north-drought",
        "underground",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto Drought-covered Rubble from a drought occupied Water site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1349_playing_water_onto_flooded_occupied_earth_rubble_creates_water_site() {
    let mut session = flood_site_type_opening();
    let site_instance = flooded_earth_site_at_c3(&mut session);
    destroy_occupied_site_at_c3(&mut session, &site_instance);
    pass_turn_and_draw_spellbook(&mut session);
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto Flood-covered Rubble from a flooded occupied Earth site must become a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1350_playing_earth_onto_drought_occupied_water_rubble_creates_earth_site() {
    let mut session = drought_occupied_water_rubble_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto Drought-covered Rubble from a drought occupied Water site must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1367_water_site_cast_minion_offered_on_flooded_occupied_earth_site() {
    let mut session = flood_site_type_opening();
    flooded_earth_site_at_c3(&mut session);
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Flood on an occupied Earth site must make that cell count as Water immediately"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1368_water_site_cast_minion_withheld_on_drought_occupied_water_site() {
    let mut session = drought_site_type_opening();
    drought_water_site_at_c3(&mut session);
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Drought on an occupied Water site must make that cell count as land immediately"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1377_playing_water_onto_flooded_occupied_earth_relayers_underwater() {
    let mut session = flood_opening_with_water_site();
    let dualer_id = cover_occupied_dualer_at_c3(
        &mut session,
        "north-earth",
        "north-earth",
        "underground",
        "north-flood",
        "underwater",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a flooded occupied Earth site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1378_playing_earth_onto_drought_occupied_water_relayers_underground() {
    let mut session = drought_earth_play_opening();
    let dualer_id = cover_occupied_dualer_at_c3(
        &mut session,
        "north-water",
        "north-water",
        "underwater",
        "north-drought",
        "underground",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a drought occupied Water site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1379_playing_water_onto_flooded_occupied_earth_creates_water_site() {
    let mut session = flood_site_type_opening();
    flooded_earth_site_at_c3(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto a flooded occupied Earth site must become a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1380_playing_earth_onto_drought_occupied_water_creates_land() {
    let mut session = drought_occupied_water_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto a drought occupied Water site must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1381_playing_earth_onto_flooded_occupied_earth_relayers_underwater() {
    let mut session = flood_occupied_earth_ready_for_earth_play();
    let dualer_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-dualer")
        .expect("dual-region minion on flooded occupied Earth")["instanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a flooded occupied Earth site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(
        occupant["region"], "underwater",
        "same-type Earth play onto a flooded occupied Earth site must create an effective Water site"
    );
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

fn flood_occupied_water_ready_for_water_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&flood_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            {
                return None;
            }
            cover_occupied_dualer_at_c3(
                &mut session,
                "north-earth",
                "north-water",
                "underwater",
                "north-flood",
                "underwater",
            );
            if offers_play_water_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_water_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Water play after flooded occupied Water site")
}

fn drought_occupied_earth_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&drought_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            {
                return None;
            }
            cover_occupied_dualer_at_c3(
                &mut session,
                "north-water",
                "north-earth",
                "underground",
                "north-drought",
                "underground",
            );
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after drought occupied Earth site")
}

fn drought_occupied_water_ready_for_water_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&drought_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            {
                return None;
            }
            cover_occupied_dualer_at_c3(
                &mut session,
                "north-earth",
                "north-water",
                "underwater",
                "north-drought",
                "underground",
            );
            if offers_play_water_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_water_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Water play after drought occupied Water site")
}

#[test]
fn rule_catalog_1387_playing_water_onto_flooded_occupied_water_relayers_underwater() {
    let mut session = flood_occupied_water_ready_for_water_play();
    let dualer_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-dualer")
        .expect("dual-region minion on flooded occupied Water")["instanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a flooded occupied Water site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1388_playing_earth_onto_drought_occupied_earth_relayers_underground() {
    let mut session = drought_occupied_earth_ready_for_earth_play();
    let dualer_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-dualer")
        .expect("dual-region minion on drought occupied Earth")["instanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a drought occupied Earth site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1389_playing_water_onto_drought_occupied_water_creates_land_and_relayers_underground()
 {
    let mut session = drought_occupied_water_ready_for_water_play();
    let dualer_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-dualer")
        .expect("dual-region minion on drought occupied Water")["instanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a drought occupied Water site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_eq!(snapshot["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto a drought occupied Water site must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1390_playing_earth_onto_flooded_occupied_water_relayers_underwater() {
    let mut session = flood_opening_with_water_site();
    let dualer_id = cover_occupied_dualer_at_c3(
        &mut session,
        "north-earth",
        "north-water",
        "underwater",
        "north-flood",
        "underwater",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a flooded occupied Water site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1392_playing_water_onto_drought_occupied_earth_relayers_underground() {
    let mut session = drought_earth_play_opening();
    let dualer_id = cover_occupied_dualer_at_c3(
        &mut session,
        "north-water",
        "north-earth",
        "underground",
        "north-drought",
        "underground",
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a drought occupied Earth site must relayer the dual-region unit"
    );
    let snapshot = state(&session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(&session);
}

fn flooded_water_occupied_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&flood_water_site_type_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().any(|card| card == "north-water")
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            {
                return None;
            }
            flooded_water_site_at_c3(&mut session);
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after flooded occupied Water site")
}

fn drought_earth_occupied_ready_for_earth_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&drought_earth_site_type_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            {
                return None;
            }
            drought_earth_site_at_c3(&mut session);
            if offers_play_earth_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_earth_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Earth play after drought occupied Earth site")
}

fn flooded_water_occupied_ready_for_water_play() -> Session {
    (1..4096)
        .find_map(|seed| {
            let mut session = Session::new(&flood_water_site_type_manifest(seed)).ok()?;
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            if !(atlas.iter().filter(|card| *card == "north-water").count() >= 2
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            {
                return None;
            }
            flooded_water_site_at_c3(&mut session);
            if offers_play_water_on_c3(&session) {
                return Some(session);
            }
            maybe_draw_site_for_play(&mut session);
            offers_play_water_on_c3(&session).then_some(session)
        })
        .expect("bounded seed with legal Water play after flooded occupied Water site")
}

#[test]
fn rule_catalog_1397_playing_earth_onto_flooded_occupied_water_creates_water_site() {
    let mut session = flooded_water_occupied_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto a flooded occupied Water site must become a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1398_playing_water_onto_drought_occupied_earth_creates_land() {
    let mut session = drought_earth_play_opening();
    cover_occupied_dualer_at_c3(
        &mut session,
        "north-water",
        "north-earth",
        "underground",
        "north-drought",
        "underground",
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto a drought occupied Earth site must become land"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1401_water_site_cast_minion_offered_on_flooded_occupied_water_site() {
    let mut session = flood_water_site_type_opening();
    flooded_water_site_at_c3(&mut session);
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Flood on an occupied Water site must make that cell count as Water immediately"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1402_water_site_cast_minion_withheld_on_drought_occupied_earth_site() {
    let mut session = drought_earth_site_type_opening();
    drought_earth_site_at_c3(&mut session);
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Drought on an occupied Earth site must make that cell count as land immediately"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1412_playing_water_onto_flooded_occupied_water_creates_water_site() {
    let mut session = flooded_water_occupied_ready_for_water_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Water played onto a flooded occupied Water site must remain a Water site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1413_playing_earth_onto_drought_occupied_earth_creates_land() {
    let mut session = drought_earth_occupied_ready_for_earth_play();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "printed Earth played onto a drought occupied Earth site must remain land"
    );
    assert_exact_replay(&session);
}

fn rain() -> Value {
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

fn flood_occupied_water_c3_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood-occupied-water-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-occupied-water-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-rain": rain(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-water",
                ],
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
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn try_accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> bool {
    try_accept_where_pair(session, predicate).is_some()
}

fn try_accept_where_pair(
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

fn try_pass_turn_and_draw_spellbook(session: &mut Session) -> bool {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")
        && try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
}

struct PendingDeathriteFloodedOccupiedWaterSetup {
    deathrite_ids: [String; 2],
    dualer_id: String,
    session: Session,
}

fn try_pending_deathrite_with_flooded_occupied_water_c3_dualer(
    encoded: &str,
) -> Option<PendingDeathriteFloodedOccupiedWaterSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C4"
    }) || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
    {
        return None;
    }
    let (summoned, _) = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underwater"
    })?;
    let dualer_id = summoned["cardInstanceId"].as_str()?.to_owned();
    if !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if realm_unit(&state(&session), &dualer_id)["region"] != "underwater" {
        return None;
    }
    if !try_pass_turn_and_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    let snapshot = state(&session);
    if !snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
    {
        return None;
    }
    if snapshot["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteFloodedOccupiedWaterSetup {
        deathrite_ids,
        dualer_id,
        session,
    })
}

fn flooded_occupied_water_c3_deathrite_dualer_seed_with(start: u32) -> (String, u32) {
    (start..start + 4096)
        .find_map(|seed| {
            let encoded = flood_occupied_water_c3_deathrite_manifest(seed);
            try_pending_deathrite_with_flooded_occupied_water_c3_dualer(&encoded)
                .map(|_| (encoded, seed))
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a dual-region minion on flooded occupied Water at C3",
        )
}

#[test]
fn rule_catalog_1471_playing_water_onto_flooded_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let (encoded, seed) = flooded_occupied_water_c3_deathrite_dualer_seed_with(1471);
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_dualer(&encoded)
        .expect("complete water-on-flooded-occupied-water Deathrite relayer setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
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
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(realm_unit(&resumed, &dualer_id)["region"], "underwater");
    if !offers_play_water_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a flooded occupied Water site after deathrite-order must relayer the dual-region unit"
    );
    let snapshot = state(session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(session);
}

fn drought_occupied_earth_c3_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-drought-occupied-earth-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-drought-occupied-earth-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-rain": rain(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
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
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PendingDeathriteDroughtOccupiedEarthSetup {
    deathrite_ids: [String; 2],
    dualer_id: String,
    session: Session,
}

fn try_pending_deathrite_with_drought_occupied_earth_c3_dualer(
    encoded: &str,
) -> Option<PendingDeathriteDroughtOccupiedEarthSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    }) || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
    {
        return None;
    }
    let (summoned, _) = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    let dualer_id = summoned["cardInstanceId"].as_str()?.to_owned();
    if !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if realm_unit(&state(&session), &dualer_id)["region"] != "underground" {
        return None;
    }
    if !try_pass_turn_and_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    let snapshot = state(&session);
    if !snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
    {
        return None;
    }
    if snapshot["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteDroughtOccupiedEarthSetup {
        deathrite_ids,
        dualer_id,
        session,
    })
}

fn drought_occupied_earth_c3_deathrite_dualer_seed_with(start: u32) -> (String, u32) {
    (start..start + 4096)
        .find_map(|seed| {
            let encoded = drought_occupied_earth_c3_deathrite_manifest(seed);
            try_pending_deathrite_with_drought_occupied_earth_c3_dualer(&encoded)
                .map(|_| (encoded, seed))
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a dual-region minion on drought occupied Earth at C3",
        )
}

#[test]
fn rule_catalog_1472_playing_earth_onto_drought_occupied_earth_at_c3_relayers_underground_after_deathrite_order()
 {
    let (encoded, seed) = drought_occupied_earth_c3_deathrite_dualer_seed_with(1472);
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_dualer(&encoded)
        .expect("complete earth-on-drought-occupied-earth Deathrite relayer setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
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
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(realm_unit(&resumed, &dualer_id)["region"], "underground");
    if !offers_play_earth_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a drought occupied Earth site after deathrite-order must relayer the dual-region unit"
    );
    let snapshot = state(session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(session);
}

fn flood_occupied_water_c3_earth_play_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-flood-occupied-water-earth-play-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-flood-occupied-water-earth-play-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-rain": rain(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-water",
                ],
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
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PendingDeathriteFloodedOccupiedWaterEarthPlaySetup {
    deathrite_ids: [String; 2],
    dualer_id: String,
    session: Session,
}

fn try_pending_deathrite_with_flooded_occupied_water_c3_dualer_for_earth_play(
    encoded: &str,
) -> Option<PendingDeathriteFloodedOccupiedWaterEarthPlaySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C4"
    }) || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
    {
        return None;
    }
    let (summoned, _) = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underwater"
    })?;
    let dualer_id = summoned["cardInstanceId"].as_str()?.to_owned();
    if !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_pass_turn_and_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if realm_unit(&state(&session), &dualer_id)["region"] != "underwater" {
        return None;
    }
    if !try_pass_turn_and_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    let snapshot = state(&session);
    if !snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
    {
        return None;
    }
    if snapshot["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-earth"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteFloodedOccupiedWaterEarthPlaySetup {
        deathrite_ids,
        dualer_id,
        session,
    })
}

fn flooded_occupied_water_c3_earth_play_deathrite_dualer_seed_with(start: u32) -> (String, u32) {
    (start..start + 4096)
        .find_map(|seed| {
            let encoded = flood_occupied_water_c3_earth_play_deathrite_manifest(seed);
            try_pending_deathrite_with_flooded_occupied_water_c3_dualer_for_earth_play(&encoded)
                .map(|_| (encoded, seed))
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a dual-region minion on flooded occupied Water at C3 and Earth in atlas",
        )
}

#[test]
fn rule_catalog_1473_playing_earth_onto_flooded_occupied_water_at_c3_relayers_underwater_after_deathrite_order()
 {
    let (encoded, seed) = flooded_occupied_water_c3_earth_play_deathrite_dualer_seed_with(1473);
    eprintln!("seed={seed}");
    let mut setup =
        try_pending_deathrite_with_flooded_occupied_water_c3_dualer_for_earth_play(&encoded)
            .expect("complete earth-on-flooded-occupied-water Deathrite relayer setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
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
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(realm_unit(&resumed, &dualer_id)["region"], "underwater");
    if !offers_play_earth_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Earth onto a flooded occupied Water site after deathrite-order must relayer the dual-region unit"
    );
    let snapshot = state(session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(session);
}

fn drought_occupied_earth_c3_water_play_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "play-site-overlay-drought-occupied-earth-water-play-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-play-site-overlay-drought-occupied-earth-water-play-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-rain": rain(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
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
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn try_pending_deathrite_with_drought_occupied_earth_c3_dualer_for_water_play(
    encoded: &str,
) -> Option<PendingDeathriteDroughtOccupiedEarthSetup> {
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_dualer(encoded)?;
    state(&setup.session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water"))
        .then_some(setup)
}

fn drought_occupied_earth_c3_water_play_deathrite_dualer_seed_with(start: u32) -> (String, u32) {
    (start..start + 4096)
        .find_map(|seed| {
            let encoded = drought_occupied_earth_c3_water_play_deathrite_manifest(seed);
            try_pending_deathrite_with_drought_occupied_earth_c3_dualer_for_water_play(&encoded)
                .map(|_| (encoded, seed))
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a dual-region minion on drought occupied Earth at C3 and Water in atlas",
        )
}

#[test]
fn rule_catalog_1474_playing_water_onto_drought_occupied_earth_at_c3_relayers_underground_after_deathrite_order()
 {
    let (encoded, seed) = drought_occupied_earth_c3_water_play_deathrite_dualer_seed_with(1474);
    eprintln!("seed={seed}");
    let mut setup =
        try_pending_deathrite_with_drought_occupied_earth_c3_dualer_for_water_play(&encoded)
            .expect("complete water-on-drought-occupied-earth Deathrite relayer setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let dualer_id = setup.dualer_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
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
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(realm_unit(&resumed, &dualer_id)["region"], "underground");
    if !offers_play_water_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "playing Water onto a drought occupied Earth site after deathrite-order must relayer the dual-region unit"
    );
    let snapshot = state(session);
    let occupant = realm_unit(&snapshot, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&snapshot, &dualer_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1487_play_site_withheld_during_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let (encoded, seed) = flooded_occupied_water_c3_earth_play_deathrite_dualer_seed_with(1487);
    eprintln!("seed={seed}");
    let setup =
        try_pending_deathrite_with_flooded_occupied_water_c3_dualer_for_earth_play(&encoded)
            .expect("complete earth-on-flooded-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1488_play_site_withheld_during_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let (encoded, seed) = drought_occupied_earth_c3_water_play_deathrite_dualer_seed_with(1488);
    eprintln!("seed={seed}");
    let setup =
        try_pending_deathrite_with_drought_occupied_earth_c3_dualer_for_water_play(&encoded)
            .expect("complete water-on-drought-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1497_play_site_earth_offered_after_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let (encoded, seed) = flooded_occupied_water_c3_earth_play_deathrite_dualer_seed_with(1497);
    eprintln!("seed={seed}");
    let mut setup =
        try_pending_deathrite_with_flooded_occupied_water_c3_dualer_for_earth_play(&encoded)
            .expect("complete earth-on-flooded-occupied-water Deathrite offered setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !offers_play_earth_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1498_play_site_water_offered_after_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let (encoded, seed) = drought_occupied_earth_c3_water_play_deathrite_dualer_seed_with(1498);
    eprintln!("seed={seed}");
    let mut setup =
        try_pending_deathrite_with_drought_occupied_earth_c3_dualer_for_water_play(&encoded)
            .expect("complete water-on-drought-occupied-earth Deathrite offered setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !offers_play_water_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    assert!(
        offers_play_water_on_c3(session),
        "after Deathrites complete, play-site Water must be offered on inverse drought occupied Earth at C3"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1513_play_site_water_offered_after_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let (encoded, seed) = flooded_occupied_water_c3_deathrite_dualer_seed_with(1513);
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_dualer(&encoded)
        .expect("complete water-on-flooded-occupied-water Deathrite offered setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !offers_play_water_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    assert!(
        offers_play_water_on_c3(session),
        "after Deathrites complete, play-site Water must be offered on same-type flooded occupied Water at C3"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1514_play_site_earth_offered_after_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let (encoded, seed) = drought_occupied_earth_c3_deathrite_dualer_seed_with(1514);
    eprintln!("seed={seed}");
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_dualer(&encoded)
        .expect("complete earth-on-drought-occupied-earth Deathrite offered setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !offers_play_earth_on_c3(session) {
        maybe_draw_site_for_play(session);
    }
    assert!(
        offers_play_earth_on_c3(session),
        "after Deathrites complete, play-site Earth must be offered on same-type drought occupied Earth at C3"
    );
    assert_exact_replay(session);
}
