//! Public proof for source-class ranged-strike prevention.

use serde_json::{Value, json};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        manifest["cards"] = json!({
            "north-avatar": {"cardType":"avatar", "attack":1, "defense":1, "drawSpell":false, "life":20},
            "south-avatar": {"cardType":"avatar", "attack":1, "defense":1, "drawSpell":false, "life":20},
            "north-site": {"cardType":"site", "elements":["earth"]},
            "south-site": {"cardType":"site", "elements":["earth"]},
            "shooter": {"cardType":"minion", "attack":2, "defense":2, "manaCost":0, "thresholds":{"air":0,"earth":0,"fire":0,"water":0}, "ranged":true, "tapToShootProjectileDamage":1, "entersCarrying":["bonus"]},
            "bonus": {"cardType":"artifact","manaCost":null,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"bearerUnitStrike":{"damageBonus":1,"destroyAfterStrike":true}},
            "target": {"cardType":"minion", "attack":1, "defense":3, "manaCost":0, "thresholds":{"air":0,"earth":0,"fire":0,"water":0}, "summonToAnySite":true, "preventsDamageFrom":"ranged-strikes"}
        });
        manifest["decks"] = json!({
            "north": {"avatar":"north-avatar", "atlas":vec!["north-site";12], "spellbook":vec!["shooter";8]},
            "south": {"avatar":"south-avatar", "atlas":vec!["south-site";12], "spellbook":vec!["target";8]}
        });
    })
}

fn accept(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .iter()
        .find(|a| predicate(&a.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "issued action; available={:?}",
                actions.iter().map(|a| &a.descriptor).collect::<Vec<_>>()
            )
        });
    let descriptor = action.descriptor.clone();
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
    (descriptor, receipt)
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay")["state"].clone()
}

fn ready() -> Session {
    let mut session = Session::new(&manifest(1)).expect("session");
    for _ in 0..2 {
        accept(&mut session, |d| {
            d["kind"] == "mulligan"
                && d["atlasOrder"] == json!([])
                && d["spellbookOrder"] == json!([])
        });
    }
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cardId"] == "north-site" && d["cell"] == "C4"
    });
    accept(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "shooter" && d["cell"] == "C4"
    });
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cardId"] == "south-site" && d["cell"] == "C1"
    });
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cardId"] == "north-site" && d["cell"] == "C3"
    });
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "target" && d["cell"] == "C3"
    });
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    session
}

#[test]
fn fixed_projectile_is_not_ranged_strike_prevention_and_is_checkpointable() {
    let mut session = ready();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| {
            a.descriptor["kind"] == "shoot-damage-projectile"
                && a.descriptor["hit"]["instanceId"].is_string()
        })
        .unwrap_or_else(|| {
            panic!(
                "fixed projectile; actions={:?}",
                session
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .map(|a| &a.descriptor)
                    .collect::<Vec<_>>()
            )
        });
    let target = action.descriptor["hit"]["instanceId"].clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("accepted")
    };
    assert!(
        receipt
            .events
            .iter()
            .any(|e| e.event_type == "projectile-damage-allocated")
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["instanceId"] == target)
            .unwrap()["damage"],
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-consumed-after-strike")
            .count(),
        0
    );
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let replay = resume_game_checkpoint(&checkpoint).unwrap();
    assert_eq!(replay.state_hash().unwrap(), session.state_hash().unwrap());
}

#[test]
fn ranged_strike_is_blocked_by_ranged_strike_prevention_and_is_checkpointable() {
    let mut session = ready();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| {
            a.descriptor["kind"] == "shoot-projectile" && a.descriptor["hit"]["kind"] == "minion"
        })
        .expect("issued ranged strike");
    let target = action.descriptor["hit"]["instanceId"].clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("accepted")
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .find(|e| e.event_type == "damage-dealt")
            .unwrap()
            .payload["amount"],
        0
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["instanceId"] == target)
            .unwrap()["damage"],
        0
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-consumed-after-strike")
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-banished")
            .count(),
        1
    );
    let replay = resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    assert_eq!(replay.state_hash().unwrap(), session.state_hash().unwrap());
}
