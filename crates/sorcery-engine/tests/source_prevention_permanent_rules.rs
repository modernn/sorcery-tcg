//! Public proof that fire-magic prevention does not suppress permanent Fire sources.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Receipt {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "engine-issued action; available={:?}",
                session
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .map(|a| &a.descriptor)
                    .collect::<Vec<_>>()
            )
        });
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("step")
    else {
        panic!("issued action rejected")
    };
    receipt
}

fn manifest(kind: &str) -> String {
    let zero = json!({"air":0,"earth":0,"fire":0,"water":0});
    let fire = json!({"air":0,"earth":0,"fire":1,"water":0});
    let cards = if kind == "strike" {
        json!({
            "north-avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "south-avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "north-site":{"cardType":"site","elements":["fire"]},
            "south-site":{"cardType":"site","elements":["fire"]},
            "source":{"cardType":"minion","attack":2,"defense":2,"manaCost":0,"ordinary":true,"elements":["fire"],"thresholds":fire,"summonToAnySite":true},
            "target":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"summonToAnySite":true,"thresholds":zero,"preventsDamageFrom":"fire-magic"}
        })
    } else {
        json!({
            "north-avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "south-avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "north-site":{"cardType":"site","elements":["fire"]},
            "south-site":{"cardType":"site","elements":["fire"]},
            "bearer":{"cardType":"minion","attack":1,"defense":3,"manaCost":0,"summonToAnySite":true,"thresholds":zero},
            "helper":{"cardType":"minion","attack":1,"defense":3,"manaCost":0,"summonToAnySite":true,"thresholds":zero},
            "source":{"cardType":"artifact","manaCost":0,"tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps":3,"thresholds":fire},
            "target":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"summonToAnySite":true,"thresholds":zero,"preventsDamageFrom":"fire-magic"}
        })
    };
    let mut value = json!({
        "schemaVersion":1,"engineVersion":"sorcery-core-v1","seed":17,"firstSeat":"north",
        "authority":{"mode":"synthetic","revisionId":"source-prevention-permanent-v1","contentHash":identity_hash(&json!({"fixture":kind})).unwrap()},
        "cards":cards,
        "decks": if kind == "strike" {
            json!({"north":{"avatar":"north-avatar","atlas":vec!["north-site";6],"spellbook":vec!["source";3]},"south":{"avatar":"south-avatar","atlas":vec!["south-site";6],"spellbook":vec!["target";3]}})
        } else {
            json!({"north":{"avatar":"north-avatar","atlas":vec!["north-site";6],"spellbook":["bearer","helper","source"]},"south":{"avatar":"south-avatar","atlas":vec!["south-site";6],"spellbook":vec!["target";3]}})
        },
    });
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn keep(session: &mut Session) {
    act(session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
}

fn damage_for(receipt: &Receipt, target: &Value) -> u64 {
    receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt"
                && (event.payload["targetInstanceId"] == *target
                    || event.payload["instanceId"] == *target)
        })
        .unwrap_or_else(|| panic!("damage event; events={:?}", receipt.events))
        .payload["amount"]
        .as_u64()
        .expect("damage amount")
}

fn unit_id(session: &Session, card_id: &str) -> Value {
    session.replay_value().unwrap()["state"]["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("unit")["instanceId"]
        .clone()
}

#[test]
fn fire_minion_strike_bypasses_fire_magic_prevention_and_replays() {
    let mut session = Session::new(&manifest("strike")).unwrap();
    keep(&mut session);
    keep(&mut session);
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "source" && d["cell"] == "C4"
    });
    act(&mut session, |d| d["kind"] == "end-turn");
    act(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "target" && d["cell"] == "C4"
    });
    let target = unit_id(&session, "target");
    act(&mut session, |d| d["kind"] == "end-turn");
    act(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    act(&mut session, |d| {
        d["kind"] == "move-and-attack" && d["to"]["cell"] == "C4"
    });
    while session
        .legal_actions()
        .unwrap()
        .iter()
        .any(|action| action.descriptor["kind"] == "continue-basic-movement")
    {
        act(&mut session, |d| d["kind"] == "continue-basic-movement");
    }
    act(&mut session, |d| {
        d["kind"] == "declare-attack" && d["target"]["instanceId"] == target
    });
    let receipt = act(&mut session, |d| {
        d["kind"] == "close-defend" && d["originalTargetParticipates"] == true
    });
    assert_eq!(damage_for(&receipt, &target), 2);
    let replay = resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    assert_eq!(replay.state_hash().unwrap(), session.state_hash().unwrap());
}

#[test]
fn fire_artifact_damage_bypasses_fire_magic_prevention_and_replays() {
    let mut session = Session::new(&manifest("artifact")).unwrap();
    keep(&mut session);
    keep(&mut session);
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "bearer" && d["cell"] == "C4"
    });
    let bearer = unit_id(&session, "bearer");
    act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "helper" && d["cell"] == "C4"
    });
    act(&mut session, |d| {
        d["kind"] == "cast-artifact"
            && d["cardId"] == "source"
            && d["bearer"]["instanceId"] == bearer
    });
    act(&mut session, |d| d["kind"] == "end-turn");
    act(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "target" && d["cell"] == "C4"
    });
    let target = unit_id(&session, "target");
    act(&mut session, |d| d["kind"] == "end-turn");
    act(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    let receipt = act(&mut session, |d| {
        d["kind"] == "activate-artifact-damage" && d["target"]["instanceId"] == target
    });
    assert_eq!(damage_for(&receipt, &target), 3);
    let replay = resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    assert_eq!(replay.state_hash().unwrap(), session.state_hash().unwrap());
}
