//! Direct proofs for start-turn controller life gain (RULE-CATALOG-0268–0269).
//!
//! Official cards can gain their controller life at the start of that player's
//! turn. Healing uses the shared Avatar helper: it is capped at printed life and
//! cannot leave Death's Door. The ability is mandatory and shares the Start
//! Phase window with other start-of-turn sources.

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

fn source() -> Value {
    json!({
        "airborne": true,
        "atStartOfControllerTurnControllerGainsLife": 2,
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

fn manifest(north_life: u8) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-life-gain" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-life-gain-v1",
        },
        "cards": {
            "north-avatar": avatar(north_life),
            "north-site": site(),
            "north-source": source(),
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

fn after_south_drains_north(north_life: u8) -> Session {
    let mut session = Session::new(&manifest(north_life)).expect("valid life-gain session");
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn resolve_start_turn_life_gain(session: &mut Session, source_id: &Value) -> Receipt {
    assert_eq!(state(session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("start-turn life-gain actions");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == *source_id
                && action.descriptor.get("lureTargetInstanceId").is_none()
        }),
        "controller life gain is the only start-turn source"
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == *source_id
    })
    .1
}

#[test]
fn rule_catalog_0268_start_turn_life_gain_heals_the_controller_avatar() {
    let mut session = after_south_drains_north(20);
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    let receipt = resolve_start_turn_life_gain(&mut session, &source_id);
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
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["cards"]["north-source"]["airborne"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0269_start_turn_life_gain_cannot_leave_deaths_door() {
    let mut session = after_south_drains_north(2);
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(before["players"]["north"]["avatar"]["deathDoorTurn"], 2);
    let receipt = resolve_start_turn_life_gain(&mut session, &source_id);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "avatar-healed"),
        "Death's Door blocks start-turn life gain"
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(after["players"]["north"]["avatar"]["deathDoorTurn"], 2);
    assert_eq!(after["turnNumber"], 3);
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
