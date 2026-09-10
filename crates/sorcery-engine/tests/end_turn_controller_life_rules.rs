//! Direct proofs for end-of-controller-turn Avatar life (RULE-CATALOG-0304–0307).
//!
//! Official minions can gain or lose their controller a printed amount of life
//! at the end of that player's turn. The pulses reuse the shared Avatar helpers:
//! healing is capped at printed life and cannot leave Death's Door, and life
//! loss can open Death's Door without ending the game. Disabled minions do not
//! pulse. The three end-turn pulses are exclusive; Ignited is not a pulse.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn gainer() -> Value {
    json!({
        "atEndOfControllerTurnControllerGainsLife": 2,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn loser() -> Value {
    json!({
        "atEndOfControllerTurnControllerLosesLife": 2,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drain() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerLosesLife": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gain_manifest(north_life: u8) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-controller-life-gain" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-controller-life-gain-v1",
        },
        "cards": {
            "north-avatar": avatar(north_life),
            "north-site": site(),
            "north-source": gainer(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drain"; 6],
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

fn loss_manifest(north_life: u8) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-controller-life-loss" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-controller-life-loss-v1",
        },
        "cards": {
            "north-avatar": avatar(north_life),
            "north-site": site(),
            "north-source": loser(),
            "south-avatar": avatar(20),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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

fn unit_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")["instanceId"]
        .clone()
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

fn summon_north_source(session: &mut Session) {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
}

fn after_south_drains_north(north_life: u8) -> Session {
    let mut session = Session::new(&gain_manifest(north_life)).expect("valid life-gain session");
    summon_north_source(&mut session);
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
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn end_north_second_turn(session: &mut Session) -> Receipt {
    let before = state(session);
    assert_eq!(before["phase"], "draw");
    assert_eq!(before["activeSeat"], "north");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn").1
}

#[test]
fn rule_catalog_0304_end_turn_controller_life_gain_heals_the_controller_avatar() {
    let mut session = after_south_drains_north(20);
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert!(before["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    let receipt = end_north_second_turn(&mut session);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 2
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-source" && unit["instanceId"] == source_id)
    );
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0305_end_turn_controller_life_gain_cannot_leave_deaths_door() {
    let mut session = after_south_drains_north(2);
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(before["players"]["north"]["avatar"]["deathDoorTurn"], 2);
    let receipt = end_north_second_turn(&mut session);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "avatar-healed"),
        "Death's Door blocks end-turn life gain"
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(after["players"]["north"]["avatar"]["deathDoorTurn"], 2);
    assert_eq!(after["turnNumber"], 4);
    assert_eq!(after["terminal"]["status"], "active");
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-source" && unit["instanceId"] == source_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0306_end_turn_controller_life_loss_reduces_the_controller_avatar() {
    let mut session = Session::new(&loss_manifest(20)).expect("valid life-loss session");
    summon_north_source(&mut session);
    let source_id = unit_id(&session, "north-source");
    let receipt = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn").1;
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 18
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-source" && unit["instanceId"] == source_id)
    );
    assert_eq!(after["players"]["north"]["avatar"]["life"], 18);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0307_end_turn_controller_life_loss_can_open_deaths_door() {
    let mut session = Session::new(&loss_manifest(2)).expect("valid Death's Door life-loss session");
    summon_north_source(&mut session);
    let source_id = unit_id(&session, "north-source");
    let receipt = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn").1;
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 0
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-reached-deaths-door"
                && event.payload["seat"] == "north"
                && event.payload["sourceInstanceId"] == source_id
                && event.payload["turnNumber"] == 1)
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(after["players"]["north"]["avatar"]["deathDoorTurn"], 1);
    assert_eq!(after["turnNumber"], 2);
    assert_eq!(after["terminal"]["status"], "active");
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-source" && unit["instanceId"] == source_id)
    );
    assert_exact_replay(&session);
}
