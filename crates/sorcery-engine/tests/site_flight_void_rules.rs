//! Direct proofs that site flight surfaces void occupants
//! (RULE-CATALOG-0351–0352).
//!
//! Playing a site and creating rubble already fill a void and surface what it
//! held. Flight onto a nearby void is the same surface fill: a Voidwalk minion
//! and a loose Artifact already in that void must surface, not stay stranded
//! until a later occupancy settle banishes the minion.

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

fn voidwalk() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "voidwalk": true,
    })
}

fn blade() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerLethal": true,
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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn voidwalk_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-flight-void-voidwalk" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-flight-void-voidwalk-v1",
        },
        "cards": {
            "north-air": breeze(),
            "north-avatar": avatar(),
            "north-breeze": breeze(),
            "north-cloud": cloud(),
            "north-voidwalk": voidwalk(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-cloud"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-voidwalk",
                    "north-breeze",
                    "north-air",
                    "north-voidwalk",
                    "north-breeze",
                    "north-air",
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

fn artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-flight-void-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-flight-void-artifact-v1",
        },
        "cards": {
            "north-air": breeze(),
            "north-avatar": avatar(),
            "north-blade": blade(),
            "north-breeze": breeze(),
            "north-cloud": cloud(),
            "north-voidwalk": voidwalk(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-cloud"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-voidwalk",
                    "north-breeze",
                    "north-air",
                    "north-blade",
                    "north-blade",
                    "north-blade",
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

fn realm_artifact<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
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

fn air_opening_hand(session: &Session) -> bool {
    let spells = opening_ids(session, "spellbook");
    spells.contains(&"north-breeze".to_owned())
        && spells.contains(&"north-air".to_owned())
        && spells.contains(&"north-voidwalk".to_owned())
}

fn voidwalk_opening() -> Session {
    (1..=4096)
        .map(voidwalk_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("site-flight void voidwalk candidate");
            air_opening_hand(&session).then_some(session)
        })
        .expect("bounded seed opening with two Air providers and Voidwalk")
}

fn artifact_opening() -> Session {
    (1..=4096)
        .map(artifact_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("site-flight void artifact candidate");
            air_opening_hand(&session).then_some(session)
        })
        .expect("bounded seed opening with two Air providers and Voidwalk")
}

fn play_cloud_and_summon_voidwalk(session: &mut Session) -> String {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-cloud"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-breeze"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-air"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-voidwalk"
            && descriptor["cell"] == "D4"
            && descriptor["region"] == "void"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Voidwalk identity")
        .to_owned()
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
fn rule_catalog_0351_flight_onto_void_surfaces_voidwalk_minion() {
    let mut session = voidwalk_opening();
    let walker_id = play_cloud_and_summon_voidwalk(&mut session);
    assert_eq!(
        realm_unit(&state(&session), &walker_id).expect("void occupant")["region"],
        "void"
    );
    let receipt = fly_cloud_to_d4(&mut session);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-banished" || event.event_type == "minion-died")
    );
    assert!(event_types(&receipt).contains(&"site-flown"));
    let current = state(&session);
    let occupant = realm_unit(&current, &walker_id).expect("surfaced Voidwalk minion");
    assert_eq!(occupant["location"], "D4");
    assert_eq!(occupant["region"], "surface");
    assert!(!cemetery_has(&current, &walker_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0352_flight_onto_void_surfaces_void_artifact() {
    let mut session = artifact_opening();
    let walker_id = play_cloud_and_summon_voidwalk(&mut session);
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
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["bearer"]["instanceId"] == walker_id.as_str()
    });
    let blade_id = cast["cardInstanceId"]
        .as_str()
        .expect("Artifact identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == walker_id.as_str()
    });
    let dropped = state(&session);
    let loose = realm_artifact(&dropped, &blade_id).expect("dropped Artifact");
    assert_eq!(loose["location"], "D4");
    assert_eq!(loose["region"], "void");
    let receipt = fly_cloud_to_d4(&mut session);
    assert!(event_types(&receipt).contains(&"site-flown"));
    let current = state(&session);
    let surfaced = realm_artifact(&current, &blade_id).expect("surfaced Artifact");
    assert_eq!(surfaced["location"], "D4");
    assert_eq!(surfaced["region"], "surface");
    assert_exact_replay(&session);
}
