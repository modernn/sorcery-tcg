//! Direct proofs that playing a site onto overlay-covered rubble relayers
//! lower-layer occupants (RULE-CATALOG-0343–0344), and that Flood/Drought
//! conversion creates the expected effective site type (RULE-CATALOG-1319,
//! RULE-CATALOG-1321, RULE-CATALOG-1323, RULE-CATALOG-1325), and same-type
//! overlay play relayers lower-layer occupants (RULE-CATALOG-1328,
//! RULE-CATALOG-1329), overlay conversion on occupied sites plus
//! replacement play after destroy (RULE-CATALOG-1338–1339,
//! RULE-CATALOG-1344–1345, RULE-CATALOG-1349–1350, RULE-CATALOG-1367–1368).
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
                    "north-water",
                    "north-water",
                    "north-earth",
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
            "north-flood": flood(),
            "north-water": water(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-destroy": destroy_site(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
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
                && spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, Water site, and water-site cast")
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
    let mut session = flood_site_type_opening();
    rubble_at_c3(&mut session, "north-earth");
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

fn cover_occupied_dualer_then_destroy_c3(
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
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    pass_turn_and_draw_spellbook(session);
    pass_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| covers_c3(descriptor, aura_id));
    assert_eq!(
        realm_unit(&state(session), &dualer_id)["region"],
        relayer_region
    );
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
    let mut session = flood_site_type_opening();
    let site_instance = flooded_earth_site_at_c3(&mut session);
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Flood on an occupied Earth site must make that cell count as Water immediately"
    );
    destroy_occupied_site_at_c3(&mut session, &site_instance);
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C3".to_owned()),
        "Flood-covered empty Rubble must not accept a water-site cast before site play"
    );
    pass_turn_and_draw_spellbook(&mut session);
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
