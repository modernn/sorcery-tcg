//! Direct proofs that site flight relayers lower-layer occupants
//! (RULE-CATALOG-0341–0342).
//!
//! Overlay Auras already convert underground and underwater when a site's
//! water-ness flips in place. Flight carries occupants to a new cell. If that
//! cell's water-ness differs from the origin, the same conversion runs so a
//! Burrowing and Submerge unit is not stranded. Flood can cover voids, so one
//! proof lands on a dry cell and the other lands on a covered empty cell.

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

fn cloud() -> Value {
    json!({
        "cardType": "site",
        "elements": ["air"],
        "flyToNearbyVoidOncePerTurnAtAirThreshold": 3,
    })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
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
        "provides": "air",
        "submerge": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn breeze() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "provides": "air",
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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-flight-layer" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-flight-layer-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-breeze": breeze(),
            "north-cloud": cloud(),
            "north-dualer": dualer(),
            "north-flood": flood(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-cloud"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-dualer",
                    "north-breeze",
                    "north-flood",
                    "north-dualer",
                    "north-breeze",
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

fn flood_covers(descriptor: &Value, included: &str, excluded: &str) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == "north-flood"
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == included))
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().all(|value| value != excluded))
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

fn opening() -> Session {
    (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("site-flight layer candidate");
            let spells = opening_ids(&session, "spellbook");
            (spells.contains(&"north-flood".to_owned())
                && spells.contains(&"north-dualer".to_owned())
                && spells.contains(&"north-breeze".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Flood, a dual-region minion, and an Air provider")
}

fn ready_to_fly(flood_includes: &str, flood_excludes: &str) -> (Session, String) {
    let mut session = opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-cloud"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dualer"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let dualer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-breeze"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        flood_covers(descriptor, flood_includes, flood_excludes)
    });
    (session, dualer_id)
}

fn fly_cloud_to_d4(session: &mut Session) -> Receipt {
    let source_id = state(session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("Cloud City identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "fly-site"
            && descriptor["sourceSiteInstanceId"] == source_id
            && descriptor["targetCell"] == "D4"
    })
    .1
}

#[test]
fn rule_catalog_0341_flight_from_flooded_site_relayers_dual_region_minion_underground() {
    let (mut session, dualer_id) = ready_to_fly("C4", "D4");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underwater"
    );
    let receipt = fly_cloud_to_d4(&mut session);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "flight onto a dry cell must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "D4");
    assert_eq!(occupant["region"], "underground");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0342_flight_onto_flooded_void_relayers_dual_region_minion_underwater() {
    let (mut session, dualer_id) = ready_to_fly("D4", "C4");
    assert_eq!(
        realm_unit(&state(&session), &dualer_id)["region"],
        "underground"
    );
    let receipt = fly_cloud_to_d4(&mut session);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died"),
        "flight onto a flooded cell must relayer the dual-region unit instead of killing it"
    );
    let current = state(&session);
    let occupant = realm_unit(&current, &dualer_id);
    assert_eq!(occupant["location"], "D4");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&current, &dualer_id));
    assert_exact_replay(&session);
}
