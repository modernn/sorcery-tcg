//! Direct proofs for start-turn controller mana gain (RULE-CATALOG-0284–0285).
//!
//! Official minions can give their controller mana at the start of that
//! player's turn. The ability is mandatory, shares the Start Phase window with
//! other start-of-turn sources, and adds to the site-produced mana already
//! present when the trigger resolves.

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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn source() -> Value {
    json!({
        "atStartOfControllerTurnControllerGainsMana": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn spend() -> Value {
    json!({
        "cardType": "magic",
        "drawSpells": 1,
        "manaCost": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "cardType": "magic",
        "drawSpells": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-mana-gain" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-mana-gain-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": source(),
            "north-spend": spend(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-source", "north-spend", "north-spend"],
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

fn after_south_ends_turn() -> Session {
    let mut session = Session::new(&manifest()).expect("valid mana-gain session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn resolve_start_turn_mana_gain(session: &mut Session, source_id: &Value) -> Receipt {
    assert_eq!(state(session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("start-turn mana-gain actions");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == *source_id
                && action.descriptor.get("lureTargetInstanceId").is_none()
        }),
        "controller mana gain is the only start-turn source"
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == *source_id
    })
    .1
}

#[test]
fn rule_catalog_0284_start_turn_mana_gain_adds_to_site_mana() {
    let mut session = after_south_ends_turn();
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["mana"], 1);
    let receipt = resolve_start_turn_mana_gain(&mut session, &source_id);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "mana-gained"
            && event.payload["amount"] == 1
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], 2);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["turnNumber"], 3);
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
fn rule_catalog_0285_start_turn_mana_gain_pays_a_two_mana_spell() {
    let mut session = after_south_ends_turn();
    let source_id = unit_id(&session, "north-source");
    resolve_start_turn_mana_gain(&mut session, &source_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before_cast = state(&session);
    assert_eq!(before_cast["phase"], "main");
    assert_eq!(before_cast["players"]["north"]["mana"], 2);
    let legal = session.legal_actions().expect("main-phase actions");
    assert!(
        legal.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-spend"
        }),
        "gained mana makes the 2-cost spell legal"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-spend"
    });
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-cast")
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], 0);
    assert_eq!(after["phase"], "main");
    assert_exact_replay(&session);
}
