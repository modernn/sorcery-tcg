//! Direct proofs that Flood, Drought, and Fate relayer lower-layer occupants
//! (RULE-CATALOG-0337–0338).
//!
//! Playing Water onto rubble already floods underground occupants. Destroying a
//! current Water site already returns underwater occupants underground. Overlay
//! Auras use the same conversion: Fate floods a covered non-Ordinary earth site,
//! and Drought makes a printed Water site land again. A Burrowing and Submerge
//! unit stays in play instead of dying as if stranded. Fate Genesis still only
//! submerges surface occupants, so these proofs stand the unit below the site
//! first. Fate can cover voids; the Fate proof keeps C4 uncovered so the Avatar
//! is not drowned.

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

fn fate() -> Value {
    json!({
        "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities": true,
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

fn fate_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-layer-fate" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-layer-fate-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dualer": dualer(),
            "north-earth": earth(),
            "north-fate": fate(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-fate",
                    "north-dualer",
                    "north-fate",
                    "north-dualer",
                    "north-fate",
                    "north-dualer",
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

fn drought_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overlay-layer-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overlay-layer-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-dualer": dualer(),
            "north-water": water(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
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

fn drought_covers_c4(descriptor: &Value) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == "north-drought"
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

fn fate_opening() -> Session {
    (1..=4096)
        .map(fate_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate overlay-layer candidate");
            let spells = opening_ids(&session, "spellbook");
            (spells.contains(&"north-fate".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Fate and a dual-region minion")
}

fn drought_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Drought overlay-layer candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && spells.contains(&"north-drought".to_owned())
                && spells.contains(&"north-dualer".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Drought, Water, and a dual-region minion")
}

#[test]
fn rule_catalog_0337_fate_relayers_burrowed_dual_region_minion_underwater() {
    let mut session = fate_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, fate_covers_c3_not_c4);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "Fate must relayer the burrowed dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C3");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0338_drought_relayers_submerged_dual_region_minion_underground() {
    let mut session = drought_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, drought_covers_c4);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "Drought must relayer the submerged dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "C4");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}
