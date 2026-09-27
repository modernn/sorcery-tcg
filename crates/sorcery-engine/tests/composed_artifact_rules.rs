//! Public proof that one synthetic Artifact can compose nearby attack obligation and doubling.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Receipt {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("issued action");
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

fn manifest(seed: u32) -> String {
    let thresholds = json!({"air":0,"earth":0,"fire":0,"water":0});
    let mut value = json!({
        "schemaVersion":1,"engineVersion":"sorcery-core-v1","seed":seed,"firstSeat":"north",
        "authority":{"mode":"synthetic","revisionId":"composed-artifact-test", "contentHash":identity_hash(&json!({"fixture":"composed-artifact-test"})).unwrap()},
        "cards": {
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":{"cardType":"site","elements":["earth"]},
            "source":{"cardType":"minion","attack":2,"defense":3,"manaCost":0,"charge":true,"thresholds":thresholds},
            "target":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"summonToAnySite":true,"thresholds":thresholds},
            "bearer":{"cardType":"minion","attack":1,"defense":3,"manaCost":0,"summonToAnySite":true,"thresholds":thresholds},
            "combined":{"cardType":"artifact","manaCost":0,"thresholds":thresholds,"nearbyMinionsMustAttackIfAble":true,"nearbyStrikesAgainstUnitsDealDoubleDamage":true},
        },
        "decks":{
            "north":{"avatar":"avatar","atlas":vec!["site"; 8],"spellbook":vec!["source"; 8]},
            "south":{"avatar":"avatar","atlas":vec!["site"; 8],"spellbook":vec!["combined","target","bearer","target","combined","target","bearer","target"]},
        },
    });
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn ready(target_cell: &str) -> Session {
    let encoded = (1..=4096)
        .map(manifest)
        .find(|candidate| {
            let session = Session::new(candidate).unwrap();
            session.replay_value().unwrap()["state"]["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "combined")
        })
        .expect("opening hand with combined artifact");
    let mut session = Session::new(&encoded).unwrap();
    for _ in 0..2 {
        act(&mut session, |a| {
            a["kind"] == "mulligan"
                && a["atlasOrder"] == json!([])
                && a["spellbookOrder"] == json!([])
        });
    }
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == "C4"
    });
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == "C1"
    });
    let bearer_card = if target_cell == "C1" {
        "bearer"
    } else {
        "target"
    };
    act(&mut session, |a| {
        a["kind"] == "summon-minion" && a["cardId"] == bearer_card && a["cell"] == target_cell
    });
    let bearer = session.replay_value().unwrap()["state"]["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["cardId"] == bearer_card)
        .unwrap()["instanceId"]
        .clone();
    act(&mut session, |a| {
        a["kind"] == "cast-artifact"
            && a["cardId"] == "combined"
            && a["bearer"]["instanceId"] == bearer
    });
    if target_cell == "C1" {
        act(&mut session, |a| {
            a["kind"] == "summon-minion" && a["cardId"] == "target" && a["cell"] == "C4"
        });
    }
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    act(&mut session, |a| {
        a["kind"] == "summon-minion" && a["cardId"] == "source" && a["cell"] == "C4"
    });
    session
}

fn resolve_attack(session: &mut Session, source: &Value, target: &Value) -> Receipt {
    act(session, |a| {
        a["kind"] == "move-and-attack" && a["unitInstanceId"] == *source && a["to"]["cell"] == "C4"
    });
    while session.replay_value().unwrap()["state"]["phase"] == "movement" {
        act(session, |a| a["kind"] == "continue-basic-movement");
    }
    act(session, |a| {
        a["kind"] == "declare-attack" && a["target"]["instanceId"] == *target
    });
    act(session, |a| {
        a["kind"] == "close-defend" && a["originalTargetParticipates"] == true
    });
    if session.replay_value().unwrap()["state"]["phase"] == "intercept" {
        act(session, |a| a["kind"] == "close-intercept");
    }
    session.transcript().last().cloned().unwrap()
}

#[test]
fn composed_artifact_obliges_attack_and_doubles_damage_with_replay() {
    let mut session = ready("C4");
    let state = session.replay_value().unwrap()["state"].clone();
    let source = state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["cardId"] == "source")
        .unwrap()["instanceId"]
        .clone();
    let target = state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["cardId"] == "target")
        .unwrap()["instanceId"]
        .clone();
    assert!(
        session
            .legal_actions()
            .unwrap()
            .iter()
            .all(|a| a.descriptor["kind"] == "move-and-attack"
                && a.descriptor["unitInstanceId"] == source)
    );
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let mut resumed = resume_game_checkpoint(
        &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
    )
    .unwrap();
    let receipt = resolve_attack(&mut session, &source, &target);
    let resumed_receipt = resolve_attack(&mut resumed, &source, &target);
    assert_eq!(receipt, resumed_receipt);
    assert!(
        session
            .transcript()
            .iter()
            .flat_map(|r| &r.events)
            .any(|e| e.event_type == "damage-dealt"
                && e.payload["instanceId"] == target
                && e.payload["amount"] == 4)
    );
    assert!(
        session.replay_value().unwrap()["state"]["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|u| u["instanceId"] != target)
    );
    assert_eq!(
        session.replay_value().unwrap(),
        resumed.replay_value().unwrap()
    );
    assert!(session.verify_replay().unwrap());
}

#[test]
fn composed_artifact_out_of_range_does_not_oblige_attack() {
    let mut session = ready("C1");
    let state = session.replay_value().unwrap()["state"].clone();
    let source = state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["cardId"] == "source")
        .unwrap()["instanceId"]
        .clone();
    let target = state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["cardId"] == "target")
        .unwrap()["instanceId"]
        .clone();
    assert!(
        session
            .legal_actions()
            .unwrap()
            .iter()
            .any(|a| a.descriptor["kind"] == "end-turn")
    );
    assert!(session.legal_actions().unwrap().iter().any(|a| {
        a.descriptor["kind"] == "move-and-attack"
            && a.descriptor["unitInstanceId"] == source
            && a.descriptor["to"]["cell"] == "C4"
    }));
    resolve_attack(&mut session, &source, &target);
    assert!(
        session
            .transcript()
            .iter()
            .flat_map(|r| &r.events)
            .any(|e| {
                e.event_type == "damage-dealt"
                    && e.payload["instanceId"] == target
                    && e.payload["amount"] == 2
            })
    );
    assert_exact_replay(&session);
}

fn assert_exact_replay(session: &Session) {
    let actions = session
        .transcript()
        .iter()
        .map(|r| r.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &actions).unwrap();
    assert_eq!(
        replayed.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
}
