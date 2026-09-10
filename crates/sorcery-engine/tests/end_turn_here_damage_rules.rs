//! Direct proofs for end-of-controller-turn here-area damage (RULE-CATALOG-0290–0291).
//!
//! Official minions can deal damage to each other unit sharing their footprint
//! at the end of their controller's turn. The pulse reuses the shared here-area
//! helper and continues through the existing end-turn deathrite path so Ignited
//! deaths and end-turn Auras still run afterward.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn pulser() -> Value {
    json!({
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor(defense: u8) -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(south_defense: u8) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-here-damage" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-here-damage-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": pulser(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(south_defense),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn unit<'a>(state: &'a Value, card_id: &str) -> &'a Value {
    state["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")
}

fn unit_id(session: &Session, card_id: &str) -> Value {
    let snapshot = state(session);
    unit(&snapshot, card_id)["instanceId"].clone()
}

fn avatar_id(session: &Session, seat: &str) -> Value {
    state(session)["players"][seat]["avatar"]["card"]["instanceId"].clone()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

fn after_north_ready_to_end_second_turn(south_defense: u8) -> Session {
    let mut session = Session::new(&manifest(south_defense)).expect("valid here-damage session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    session
}

fn allocated_targets(receipt: &Receipt, source_id: &Value) -> Vec<Value> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "end-turn-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 1);
            assert_eq!(event.payload["sourceInstanceId"], *source_id);
            event.payload["targetInstanceId"].clone()
        })
        .collect()
}

#[test]
fn rule_catalog_0290_end_turn_here_damage_hits_other_units_sharing_the_cell() {
    let mut session = after_north_ready_to_end_second_turn(2);
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let north_avatar_id = avatar_id(&session, "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let mut targets = allocated_targets(&receipt, &source_id);
    targets.sort_by_key(Value::to_string);
    let mut expected = vec![north_avatar_id, visitor_id.clone()];
    expected.sort_by_key(Value::to_string);
    assert_eq!(targets, expected);
    assert!(
        receipt.events.iter().all(|event| {
            event.payload.get("targetInstanceId") != Some(&source_id)
                || event.event_type != "end-turn-damage-allocated"
        }),
        "the pulser does not damage itself"
    );
    let after = state(&session);
    let pulser = unit(&after, "north-pulser");
    assert_eq!(pulser["instanceId"], source_id);
    assert_eq!(pulser["damage"], 0);
    assert_eq!(unit(&after, "south-visitor")["damage"], 1);
    assert_eq!(unit(&after, "south-visitor")["instanceId"], visitor_id);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["turnNumber"], 4);
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("end-turn here-damage checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized pulse");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed pulse");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed pulse session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0291_end_turn_here_damage_destroys_a_wounded_visitor() {
    let mut session = after_north_ready_to_end_second_turn(1);
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "end-turn-damage-allocated"
            && event.payload["targetInstanceId"] == visitor_id
    }));
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died")
    );
    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-pulser" && unit["instanceId"] == source_id)
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["cardId"] != "south-visitor")
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["cardId"] == "south-visitor" && card["instanceId"] == visitor_id)
    );
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
}
