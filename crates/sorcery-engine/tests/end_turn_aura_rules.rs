//! Direct proof for end-turn Aura random damage (RULE-CATALOG-0065).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut base = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-aura-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-aura-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["spell-0", "spell-1", "spell-2", "spell-3", "spell-4", "spell-5"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["spell-6"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    for spell in 0..=6 {
        base["cards"][format!("spell-{spell}")] = minion();
    }
    let preview = Session::new(
        &canonical_json(&{
            let mut value = base.clone();
            value["manifestId"] = json!(identity_hash(&value).expect("preview identity"));
            value
        })
        .expect("preview manifest"),
    )
    .expect("preview session");
    let preview_state = preview.replay_value().expect("preview state");
    let hand = preview_state["state"]["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    let [charm_id, aura_id, minion_id]: [&str; 3] = hand
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("three opening spellbook cards");
    base["cards"][charm_id] = json!({
        "bearerControllerChoosesExtraRandomOutcome": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    base["cards"][aura_id] = json!({
        "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep": 3,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    base["cards"][minion_id] = minion();
    base["manifestId"] = json!(identity_hash(&base).expect("manifest identity"));
    canonical_json(&base).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn setup(seed: u32) -> Option<(Session, Session)> {
    let manifest_json = manifest(seed);
    let preview = Session::new(&manifest_json).ok()?;
    let preview_state = preview.replay_value().ok()?;
    let hand = preview_state["state"]["players"]["north"]["hand"]["spellbook"].as_array()?;
    let charm_id = hand.first()?["cardId"].as_str()?;
    let aura_id = hand.get(1)?["cardId"].as_str()?;
    let minion_id = hand.get(2)?["cardId"].as_str()?;

    let mut session = Session::new(&manifest_json).ok()?;
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == charm_id
            && descriptor["bearer"]["kind"] == "avatar"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == minion_id
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == aura_id
            && descriptor["cells"] == json!(["B3", "B4", "C3", "C4"])
    });
    let before_end_turn = session.clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let random_count = session
        .legal_actions()
        .ok()?
        .iter()
        .filter(|action| action.descriptor["kind"] == "resolve-end-turn-aura-random")
        .count();
    (random_count == 2).then_some((before_end_turn, session))
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves random damage before optional move and forged rejection"
)]
fn rule_catalog_0065_end_turn_aura_damages_random_unit_before_optional_move() {
    let (before_end_turn, mut session) = (2..=100)
        .find_map(setup)
        .expect("seed with two Lucky Charm end-turn outcomes");
    let random_actions: Vec<_> = session
        .legal_actions()
        .expect("end-turn aura actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-end-turn-aura-random")
        .collect();
    assert_eq!(random_actions.len(), 2);
    assert!(
        !before_end_turn
            .legal_actions()
            .expect("pre-end-turn actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "resolve-end-turn-aura-random")
    );
    assert_eq!(state(&session)["phase"], "end-turn-aura");
    let committed = session.transcript().last().expect("end-turn receipt");
    assert!(
        !committed
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt")
    );
    assert!(
        !committed
            .events
            .iter()
            .any(|event| event.event_type == "turn-ended")
    );
    assert_eq!(committed.random_draws.len(), 2);
    assert!(
        committed
            .random_draws
            .iter()
            .all(|draw| draw["purpose"] == "aura_end_turn_random_unit_at_affected_sites")
    );

    let aura = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("aura identity")
        .to_owned();
    let chosen = random_actions[0].clone();
    let chosen_id = chosen.descriptor["outcomeInstanceId"]
        .as_str()
        .expect("chosen target")
        .to_owned();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: chosen.action_id.to_string(),
            seat: chosen.seat,
            state_version: chosen.state_version,
        })
        .expect("resolve random")
    else {
        panic!("chosen random outcome must be accepted");
    };
    assert!(receipt.random_draws.is_empty());
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "aura-end-turn-damage-allocated"
            && event.payload["targetInstanceId"] == chosen_id
    }));
    assert_eq!(state(&session)["phase"], "end-turn-aura");

    let moves: Vec<_> = session
        .legal_actions()
        .expect("move actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-end-turn-aura-move")
        .collect();
    assert_eq!(moves.len(), 4);

    let forged = session
        .step(ActionRequest {
            action_id: identity_hash(&json!({
                "descriptor": {
                    "auraInstanceId": aura,
                    "cells": ["A1", "A2", "B1", "B2"],
                    "kind": "resolve-end-turn-aura-move",
                },
                "engineVersion": "sorcery-core-v1",
                "seat": "north",
                "stateVersion": state(&session)["stateVersion"],
            }))
            .expect("forged action id")
            .to_string(),
            seat: Seat::North,
            state_version: state(&session)["stateVersion"]
                .as_u64()
                .expect("state version"),
        })
        .expect("forged step");
    assert!(matches!(
        forged,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::UnknownAction
    ));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-end-turn-aura-move"
            && descriptor["cells"] == json!(["C3", "C4", "D3", "D4"])
    });
    assert_eq!(
        state(&session)["realm"]["auras"][0]["cells"],
        json!(["C3", "C4", "D3", "D4"])
    );
    assert_eq!(state(&session)["phase"], "draw");
    assert!(
        session
            .transcript()
            .last()
            .expect("turn ended")
            .events
            .iter()
            .any(|event| event.event_type == "turn-ended")
    );

    for _ in 2..=3 {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if !state(&session)["players"]["south"]["domainEstablished"]
            .as_bool()
            .unwrap_or(false)
        {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "resolve-end-turn-aura-random"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "resolve-end-turn-aura-move" && descriptor.get("cells").is_none()
        });
    }

    assert_eq!(state(&session)["realm"]["auras"], Value::Null);
    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura),
        "dispelled aura returns to cemetery"
    );
    assert!(session.verify_replay().expect("verified replay"));
}
