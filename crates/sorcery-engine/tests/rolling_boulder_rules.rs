//! Direct proof for Rolling Boulder path push damage (RULE-CATALOG-0145).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat, opaque_action_id};
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(revision: &str, cards: &Value, decks: &Value, seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rolling-boulder-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": revision,
        },
        "cards": cards,
        "decks": decks,
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn rolling_boulder_scenario() -> String {
    let thresholds = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let cards = json!({
        "boulder-north-avatar": avatar(),
        "boulder-north-site": { "cardType": "site", "elements": ["earth"] },
        "boulder-off-path-target": minion(json!({ "defense": 5 })),
        "boulder-south-filler": minion(json!({ "defense": 1 })),
        "boulder-origin-target": minion(json!({ "defense": 4 })),
        "boulder-pusher": minion(json!({
            "defense": 5,
            "lanceCount": 1,
            "lethal": true,
            "stealth": true,
        })),
        "boulder-south-avatar": avatar(),
        "boulder-south-site": { "cardType": "site", "elements": ["earth"] },
        "boulder-warded-target": minion(json!({ "defense": 5, "stealth": true, "ward": true })),
        "rolling-boulder": {
            "cardType": "artifact",
            "manaCost": 0,
            "tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath": 4,
            "thresholds": thresholds,
        },
    });
    let south_atlas = vec!["boulder-south-site"; 6];
    let decks = json!({
        "north": {
            "atlas": vec!["boulder-north-site"; 6],
            "avatar": "boulder-north-avatar",
            "spellbook": ["rolling-boulder", "boulder-pusher", "boulder-origin-target"],
        },
        "south": {
            "atlas": south_atlas,
            "avatar": "boulder-south-avatar",
            "spellbook": ["boulder-warded-target", "boulder-off-path-target", "boulder-south-filler"],
        },
    });
    manifest("synthetic-rolling-boulder-v1", &cards, &decks, 1)
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    accept_where_label(session, "engine-issued action", predicate)
}

fn accept_where_label(
    session: &mut Session,
    label: &str,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!("expected {label}");
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

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
}

fn descriptors_of_kind(session: &Session, kind: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| descriptor["kind"] == kind)
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

struct BoulderPosition {
    boulder: String,
    origin_target: String,
    pusher: String,
    warded: String,
    off_path: String,
}

fn boulder_position(session: &mut Session) -> BoulderPosition {
    keep(session);
    keep(session);
    accept_where_label(session, "play North site on C4", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let pusher = accept_where_label(session, "summon boulder-pusher on C4", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "boulder-pusher"
            && descriptor["cell"] == "C4"
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("pusher identity")
        .to_owned();
    let origin_target = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "boulder-origin-target"
            && descriptor["cell"] == "C4"
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("origin target identity")
        .to_owned();
    accept_where_label(session, "cast loose Rolling Boulder on C4", |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "rolling-boulder"
            && (descriptor.get("bearer").is_none() || descriptor["bearer"].is_null())
            && descriptor["cell"] == "C4"
    });
    let boulder = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("Boulder identity")
        .to_owned();
    assert!(
        descriptors_of_kind(session, "activate-artifact-roll-damage")
            .iter()
            .all(|descriptor| descriptor["pusher"]["instanceId"] != pusher),
        "a summoning-sick pusher cannot roll the Boulder yet"
    );
    end_and_draw(session, "atlas");

    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(session, "atlas");

    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    end_and_draw(session, "atlas");

    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let warded = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "boulder-warded-target"
            && descriptor["cell"] == "C2"
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("warded identity")
        .to_owned();
    end_and_draw(session, "atlas");

    end_and_draw(session, "atlas");

    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B1"
    });
    let off_path = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "boulder-off-path-target"
            && descriptor["cell"] == "B1"
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("off-path identity")
        .to_owned();
    end_and_draw(session, "atlas");

    BoulderPosition {
        boulder,
        origin_target,
        pusher,
        warded,
        off_path,
    }
}

fn roll_paths(session: &Session, boulder: &str, pusher: &str) -> Vec<(String, Vec<String>)> {
    descriptors_of_kind(session, "activate-artifact-roll-damage")
        .into_iter()
        .filter(|descriptor| {
            descriptor["artifactInstanceId"] == boulder
                && descriptor["pusher"]["instanceId"] == pusher
        })
        .map(|descriptor| {
            (
                descriptor["direction"]
                    .as_str()
                    .expect("direction")
                    .to_owned(),
                descriptor["path"]
                    .as_array()
                    .expect("path")
                    .iter()
                    .map(|location| location["cell"].as_str().expect("path cell").to_owned())
                    .collect(),
            )
        })
        .collect()
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one scenario proof keeps setup, forged path, and replay together"
)]
fn rule_catalog_0145_rolling_boulder_should_roll_maximally_and_damage_other_units_along_its_path() {
    let mut session =
        Session::new(&rolling_boulder_scenario()).expect("valid Rolling Boulder scenario");
    let position = boulder_position(&mut session);

    assert_eq!(
        roll_paths(&session, &position.boulder, &position.pusher),
        [
            ("east".to_owned(), vec!["C4".to_owned()]),
            ("north".to_owned(), vec!["C4".to_owned()]),
            (
                "south".to_owned(),
                vec![
                    "C4".to_owned(),
                    "C3".to_owned(),
                    "C2".to_owned(),
                    "C1".to_owned(),
                ],
            ),
            ("west".to_owned(), vec!["C4".to_owned()]),
        ]
    );

    let south_roll = descriptors_of_kind(&session, "activate-artifact-roll-damage")
        .into_iter()
        .find(|descriptor| {
            descriptor["artifactInstanceId"] == position.boulder
                && descriptor["pusher"]["instanceId"] == position.pusher
                && descriptor["direction"] == "south"
        })
        .expect("south roll")
        .clone();

    let mut shortened = south_roll.clone();
    shortened["path"] = json!(south_roll["path"].as_array().expect("path")[..2]);
    let state_version = session.replay_value().expect("replay")["state"]["stateVersion"]
        .as_u64()
        .expect("state version");
    let forged = session
        .step(ActionRequest {
            action_id: opaque_action_id("sorcery-core-v1", Seat::North, state_version, &shortened)
                .expect("forged action identity")
                .to_string(),
            seat: Seat::North,
            state_version,
        })
        .expect("forged step");
    assert!(
        matches!(forged, StepResult::Rejected { .. }),
        "a truncated path must stay illegal"
    );

    let mut carried = Session::new(&rolling_boulder_scenario()).expect("carried scenario");
    let carried_position = boulder_position(&mut carried);
    accept_where(&mut carried, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == carried_position.pusher
            && descriptor["artifactInstanceIds"] == json!([carried_position.boulder])
    });
    accept_where(&mut carried, |descriptor| {
        descriptor["kind"] == "activate-artifact-roll-damage"
            && descriptor["artifactInstanceId"] == carried_position.boulder
            && descriptor["pusher"]["instanceId"] == carried_position.pusher
            && descriptor["direction"] == "south"
    });
    let carried_artifact = &state(&carried)["realm"]["artifacts"][0];
    assert_eq!(carried_artifact["location"], "C1");
    assert_eq!(carried_artifact["region"], "surface");
    assert!(carried_artifact.get("bearer").is_none());

    let zero_receipt = {
        let mut zero_session =
            Session::new(&rolling_boulder_scenario()).expect("zero-roll scenario");
        let zero_position = boulder_position(&mut zero_session);
        let north_roll = descriptors_of_kind(&zero_session, "activate-artifact-roll-damage")
            .into_iter()
            .find(|descriptor| {
                descriptor["artifactInstanceId"] == zero_position.boulder
                    && descriptor["pusher"]["instanceId"] == zero_position.pusher
                    && descriptor["direction"] == "north"
            })
            .expect("north roll")
            .clone();
        let receipt = accept_where(&mut zero_session, |descriptor| descriptor == &north_roll).1;
        assert_eq!(
            state(&zero_session)["realm"]["artifacts"][0]["location"],
            "C4"
        );
        assert_eq!(
            realm_unit(&state(&zero_session), &zero_position.origin_target).expect("origin target")
                ["damage"],
            0
        );
        assert_exact_replay(&zero_session);
        receipt
    };

    assert_eq!(
        zero_receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["artifact-roll-damage-activated"]
    );

    let (_, south_receipt) = accept_where(&mut session, |descriptor| descriptor == &south_roll);
    let settled = state(&session);
    let surviving_pusher = realm_unit(&settled, &position.pusher).expect("pusher");
    let surviving_ward = realm_unit(&settled, &position.warded).expect("warded");
    assert_eq!(settled["players"]["north"]["avatar"]["life"], 16);
    assert_eq!(settled["players"]["south"]["avatar"]["life"], 16);
    assert_eq!(
        realm_unit(&settled, &position.off_path).expect("off-path target")["damage"],
        0
    );
    assert!(realm_unit(&settled, &position.origin_target).is_none());
    assert_eq!(
        (
            &surviving_pusher["damage"],
            &surviving_pusher["carriedLanceCount"],
            &surviving_pusher["stealthed"],
            &surviving_pusher["tapped"],
        ),
        (&json!(0), &json!(1), &json!(true), &json!(true))
    );
    assert_eq!(
        (
            &surviving_ward["damage"],
            &surviving_ward["stealthed"],
            &surviving_ward["warded"],
        ),
        (&json!(0), &json!(true), &json!(false))
    );
    assert!(
        settled["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == position.origin_target)
    );
    let loose_boulder = &settled["realm"]["artifacts"][0];
    assert_eq!(loose_boulder["location"], "C1");
    assert_eq!(loose_boulder["region"], "surface");
    assert!(loose_boulder.get("bearer").is_none());
    assert_eq!(
        south_receipt.events[0].event_type.as_str(),
        "artifact-roll-damage-activated"
    );
    assert!(
        canonical_json(&south_receipt.events[0].payload)
            .expect("activated payload")
            .contains("C4")
    );
    assert!(
        canonical_json(&south_receipt.events[0].payload)
            .expect("activated payload")
            .contains("C1")
    );
    assert!(!south_receipt.events.iter().any(|event| {
        matches!(
            event.event_type.as_str(),
            "fight-started" | "strike-damage-allocated" | "lance-broken" | "lethal-damage"
        )
    }));
    assert_eq!(zero_receipt.random_draws.len(), 0);
    assert!(
        session
            .transcript()
            .iter()
            .all(|receipt| receipt.random_draws.is_empty())
    );
    assert_exact_replay(&session);
}
