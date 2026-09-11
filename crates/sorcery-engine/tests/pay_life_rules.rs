//! Direct proofs for official Pay Life (RULE-CATALOG-0320–0321).
//!
//! Paying life is an additional Magic cost, not losing life or damage. The
//! caster may pay only when current life is at least the printed amount, so
//! Death's Door cannot pay. The cost is paid before the cast is announced.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn pay_life_heal(amount: u8) -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "payLifeAsAdditionalCost": amount,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32, life: u8, spell_count: usize) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "pay-life-cost" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-pay-life-cost-v1",
        },
        "cards": {
            "north-avatar": avatar(life),
            "north-pay": pay_life_heal(2),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pay"; spell_count],
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

fn opening_main(seed: u32, life: u8) -> Session {
    let mut session = Session::new(&manifest(seed, life, 6)).expect("valid Pay Life scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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

fn offers_pay_life(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-pay"
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

#[test]
fn rule_catalog_0320_pay_life_cost_is_paid_before_the_cast_resolves() {
    let mut session = opening_main(320, 20);
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 20);
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-pay"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("Pay Life identity")
        .to_owned();
    assert_eq!(
        event_types(&receipt),
        ["life-paid", "magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(receipt.events[0].payload["amount"], 2);
    assert_eq!(receipt.events[0].payload["life"], 18);
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[1].payload["lifePaid"], 2);
    assert_eq!(receipt.events[1].payload["instanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["amount"], 1);
    assert_eq!(receipt.events[2].payload["life"], 19);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-life-lost")
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(session.observe(Seat::South), south_observation);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0321_deaths_door_cannot_pay_life() {
    let blocked = opening_main(321, 1);
    assert_eq!(state(&blocked)["players"]["north"]["avatar"]["life"], 1);
    assert!(!offers_pay_life(&blocked));
    assert_exact_replay(&blocked);

    let mut session = opening_main(322, 2);
    assert!(offers_pay_life(&session));
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-pay"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("Pay Life identity")
        .to_owned();
    assert_eq!(
        event_types(&receipt),
        [
            "life-paid",
            "avatar-reached-deaths-door",
            "magic-cast",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[0].payload["amount"], 2);
    assert_eq!(receipt.events[0].payload["life"], 0);
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["lifePaid"], 2);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-life-lost"
                || event.event_type == "avatar-healed"
                || event.event_type == "avatar-defeated")
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert!(!after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert!(!offers_pay_life(&session));
    assert_exact_replay(&session);
}
