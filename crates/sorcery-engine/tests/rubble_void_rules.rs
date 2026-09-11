//! Direct proofs that Geomancer adjacent rubble surfaces void occupants
//! (RULE-CATALOG-0349–0350).
//!
//! Playing a site already fills the void and surfaces what it held. Adjacent
//! rubble created with an earth site is the same surface fill: a Voidwalk
//! minion and a loose Artifact already in that void must surface, not stay
//! stranded until a later occupancy settle banishes the minion.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(geomancer: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    if geomancer {
        value["earthSitePlayCreatesAdjacentRubble"] = json!(true);
    }
    value
}

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn water() -> Value {
    json!({ "cardType": "site", "elements": ["water"] })
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

fn waterbound_voidwalk() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "voidwalk": true,
        "waterbound": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn voidwalk_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rubble-void-voidwalk" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rubble-void-voidwalk-v1",
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-earth": earth(),
            "north-voidwalk": voidwalk(),
            "south-avatar": avatar(false),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-voidwalk"; 8],
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

fn artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rubble-void-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rubble-void-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-blade": blade(),
            "north-earth": earth(),
            "north-voidwalk": waterbound_voidwalk(),
            "north-water": water(),
            "south-avatar": avatar(false),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": ["north-water", "north-earth", "north-earth", "north-earth", "north-earth", "north-earth"],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-voidwalk",
                    "north-blade",
                    "north-voidwalk",
                    "north-blade",
                    "north-voidwalk",
                    "north-blade",
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

fn voidwalk_opening() -> Session {
    (1..=4096)
        .map(voidwalk_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("rubble-void voidwalk candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-voidwalk".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth and Voidwalk")
}

fn artifact_opening() -> Session {
    (1..=4096)
        .map(artifact_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("rubble-void artifact candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            (atlas.contains(&"north-water".to_owned())
                && atlas.contains(&"north-earth".to_owned())
                && spells.contains(&"north-voidwalk".to_owned())
                && spells.contains(&"north-blade".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Water, earth, Voidwalk, and an Artifact")
}

#[test]
fn rule_catalog_0349_adjacent_rubble_surfaces_voidwalk_minion() {
    let mut session = voidwalk_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C4"
            && descriptor["createRubbleAt"] == "D4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-voidwalk"
            && descriptor["cell"] == "B4"
            && descriptor["region"] == "void"
    });
    let walker_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Voidwalk identity")
        .to_owned();
    assert_eq!(
        realm_unit(&state(&session), &walker_id).expect("void occupant")["region"],
        "void"
    );
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
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
            && descriptor["createRubbleAt"] == "B4"
    });
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-banished" || event.event_type == "minion-died")
    );
    assert!(event_types(&receipt).contains(&"rubble-created"));
    let current = state(&session);
    let occupant = realm_unit(&current, &walker_id).expect("surfaced Voidwalk minion");
    assert_eq!(occupant["location"], "B4");
    assert_eq!(occupant["region"], "surface");
    assert!(!cemetery_has(&current, &walker_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0350_adjacent_rubble_surfaces_void_artifact() {
    let mut session = artifact_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-voidwalk"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let walker_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Voidwalk identity")
        .to_owned();
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
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker_id.as_str()
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "B4", "region": "void" },
                ])
    });
    let dropped = state(&session);
    let loose = realm_artifact(&dropped, &blade_id).expect("dropped Artifact");
    assert_eq!(loose["location"], "B4");
    assert_eq!(loose["region"], "void");
    if !session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "end-turn")
    {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["createRubbleAt"] == "B4"
    });
    assert!(event_types(&receipt).contains(&"rubble-created"));
    let current = state(&session);
    let surfaced = realm_artifact(&current, &blade_id).expect("surfaced Artifact");
    assert_eq!(surfaced["location"], "B4");
    assert_eq!(surfaced["region"], "surface");
    assert_exact_replay(&session);
}
