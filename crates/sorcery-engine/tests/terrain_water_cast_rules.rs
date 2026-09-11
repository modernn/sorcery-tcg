//! Direct proofs that Flood and Drought gate water-site casts
//! (RULE-CATALOG-0363–0364).
//!
//! `mustBeCastToWaterSite` uses current water-ness, not the printed element.
//! Flood on printed earth makes that site legal. Drought on printed Water
//! makes that site illegal. Ordinary land casts stay on Droughted Water.

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

fn drowned() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "mustBeCastSubmerged": true,
        "submerge": true,
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

fn flood_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-water-cast-flood" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-water-cast-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drowned": drowned(),
            "north-earth": earth(),
            "north-flood": flood(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-water-cast",
                    "north-drowned",
                    "north-flood",
                    "north-water-cast",
                    "north-drowned",
                    "north-flood",
                    "north-water-cast",
                    "north-drowned",
                    "north-flood",
                    "north-water-cast",
                    "north-drowned",
                    "north-flood",
                    "north-water-cast",
                    "north-drowned",
                    "north-flood",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 16],
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
            "contentHash": identity_hash(&json!({ "fixture": "terrain-water-cast-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-water-cast-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-water": water(),
            "north-water-cast": water_cast(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-water-cast",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-water-cast",
                    "north-drought",
                    "north-water-cast",
                    "north-water-cast",
                    "north-drought",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 16],
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
            let current = session.replay_value().expect("replay");
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                current["state"]["phase"],
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

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
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

fn summon_regions(session: &Session, card_id: &str, cell: &str) -> Vec<String> {
    let mut regions = session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == card_id
                && action.descriptor["cell"] == cell
        })
        .map(|action| {
            action.descriptor["region"]
                .as_str()
                .unwrap_or("surface")
                .to_owned()
        })
        .collect::<Vec<_>>();
    regions.sort();
    regions.dedup();
    regions
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
            let session = Session::new(&candidate).expect("Flood water-cast candidate");
            let spells = opening_ids(&session, "spellbook");
            (spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-water-cast".to_owned())
                && spells.contains(&"north-drowned".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, a water-site cast, and a submerged-only minion")
}

fn drought_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought water-cast candidate");
            let spells = opening_ids(&session, "spellbook");
            (spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-water-cast".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought and a water-site cast")
}

#[test]
fn rule_catalog_0363_flood_enables_water_site_and_underwater_casts_on_earth() {
    let mut session = flood_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C4".to_owned()),
        "printed earth must not accept a water-site cast before Flood"
    );
    assert!(
        !summon_regions(&session, "north-drowned", "C4").contains(&"underwater".to_owned()),
        "printed earth must not accept a submerged-only cast before Flood"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C4".to_owned()),
        "Flood must make printed earth a legal water-site cast, got {:?}",
        summon_cells(&session, "north-water-cast")
    );
    assert!(
        summon_regions(&session, "north-drowned", "C4").contains(&"underwater".to_owned()),
        "Flood must make printed earth a legal underwater cast, got {:?}",
        summon_regions(&session, "north-drowned", "C4")
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-water-cast"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-drowned"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0364_drought_strips_water_site_casts_from_printed_water() {
    let mut session = drought_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert!(
        summon_cells(&session, "north-water-cast").contains(&"C4".to_owned()),
        "printed Water must accept a water-site cast before Drought"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C4")
    });
    assert!(
        !summon_cells(&session, "north-water-cast").contains(&"C4".to_owned()),
        "Drought must strip water-site casts from printed Water, got {:?}",
        summon_cells(&session, "north-water-cast")
    );
    assert_exact_replay(&session);
}
