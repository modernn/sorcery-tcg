//! Legal-action, checkpoint and replay proofs for shared damage replacement ordering.
use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::session::{Session, StepResult};

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Receipt {
    let action = session
        .legal_actions()
        .expect("actions")
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

fn kind(session: &mut Session, name: &str) {
    act(session, |a| a["kind"] == name);
}

fn manifest(first: &str, bonuses: usize, step_after: bool) -> String {
    let thresholds = json!({"air":0,"earth":0,"fire":0,"water":0});
    let deck = json!({"avatar":"avatar", "atlas": vec!["site"; 8], "spellbook":vec!["shooter"; 8]});
    let mut value = json!({
        "schemaVersion":1,"engineVersion":"sorcery-core-v1","seed":71,"firstSeat":first,
        "authority":{"mode":"synthetic","revisionId":"damage-order-test", "contentHash":identity_hash(&json!({"fixture":"damage-order-test"})).unwrap()},
        "cards": {
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":{"cardType":"site","elements":["earth"]},
            "shooter":{"cardType":"minion","attack":2,"defense":3,"manaCost":0,"thresholds":thresholds,"ranged":true,"entersCarrying":["bonus","double"]},
            "bonus":{"cardType":"artifact","token":true,"manaCost":null,"thresholds":thresholds,"bearerUnitStrike":{"damageBonus":1,"destroyAfterStrike":true}},
            "double":{"cardType":"artifact","token":true,"manaCost":null,"thresholds":thresholds,"nearbyStrikesAgainstUnitsDealDoubleDamage":true},
        },
        "decks":{"north":deck,"south":deck},
    });
    let mut equipment = vec!["bonus"; bonuses];
    equipment.push("double");
    value["cards"]["shooter"]["entersCarrying"] = json!(equipment);
    if step_after {
        value["cards"]["shooter"]["mayStepAfterRangedStrike"] = json!(true);
    }
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn pending(first: &str, bonuses: usize, step_after: bool) -> Session {
    let mut session = Session::new(&manifest(first, bonuses, step_after)).expect("manifest");
    for _ in 0..2 {
        act(&mut session, |a| {
            a["kind"] == "mulligan"
                && a["atlasOrder"] == json!([])
                && a["spellbookOrder"] == json!([])
        });
    }
    let cells = if first == "north" {
        ["C4", "C3", "C2"]
    } else {
        ["C1", "C2", "C3"]
    };
    let enemy_home = if first == "north" { "C1" } else { "C4" };
    for (index, cell) in cells.into_iter().enumerate() {
        if index > 0 {
            act(&mut session, |a| {
                a["kind"] == "draw" && a["zone"] == "atlas"
            });
        }
        act(&mut session, |a| {
            a["kind"] == "play-site" && a["cell"] == cell
        });
        if index == 2 {
            act(&mut session, |a| {
                a["kind"] == "summon-minion" && a["cell"] == cell
            });
        }
        kind(&mut session, "end-turn");
        act(&mut session, |a| {
            a["kind"] == "draw" && a["zone"] == "atlas"
        });
        if index == 0 {
            act(&mut session, |a| {
                a["kind"] == "play-site" && a["cell"] == enemy_home
            });
        }
        kind(&mut session, "end-turn");
    }
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    let enemy = if first == "north" { "south" } else { "north" };
    let receipt = act(&mut session, |a| {
        a["kind"] == "shoot-projectile" && a["hit"]["kind"] == "avatar" && a["hit"]["seat"] == enemy
    });
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "strike-damage-allocated")
    );
    let state = session.replay_value().unwrap();
    assert_eq!(state["state"]["phase"], "damage-order");
    for viewer in [Seat::North, Seat::South] {
        assert_eq!(
            session.public_view(viewer).unwrap()["pendingDamageOrder"],
            state["state"]["pendingDamageOrder"]
        );
    }
    assert_eq!(state["state"]["players"][enemy]["avatar"]["life"], 20);
    session
}

#[test]
fn both_orderings_survive_checkpoint_and_replay_for_both_seats() {
    for first in ["north", "south"] {
        let original = pending(first, 1, false);
        let parallel = std::thread::scope(|scope| {
            let handles = (0..4)
                .map(|worker| {
                    let mut branch = original.clone();
                    scope.spawn(move || {
                        act(&mut branch, |a| {
                            a["kind"] == "choose-damage-modifier"
                                && a["modifierIndex"] == worker % 2
                        });
                        assert!(branch.verify_replay().unwrap());
                        branch.replay_value().unwrap()
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(parallel[0], parallel[2]);
        assert_eq!(parallel[1], parallel[3]);
        assert_ne!(parallel[0], parallel[1]);
        let cp = create_game_checkpoint(&original).unwrap();
        let parsed = parse_game_checkpoint(&serialize_game_checkpoint(&cp).unwrap()).unwrap();
        for (index, expected) in [(0, 6), (1, 5)] {
            let mut resumed = resume_game_checkpoint(&parsed).unwrap();
            let mut branch = original.clone();
            let receipt = act(&mut resumed, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            });
            let other = act(&mut branch, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            });
            assert_eq!(receipt, other);
            assert_eq!(
                resumed.replay_value().unwrap(),
                branch.replay_value().unwrap()
            );
            let allocated = receipt
                .events
                .iter()
                .find(|event| event.event_type == "strike-damage-allocated")
                .unwrap();
            assert_eq!(allocated.payload["amount"], expected);
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "artifact-consumed-after-strike")
                    .count(),
                1
            );
            let enemy = if first == "north" { "south" } else { "north" };
            assert_eq!(
                resumed.replay_value().unwrap()["state"]["players"][enemy]["avatar"]["life"],
                20 - expected
            );
            assert!(resumed.verify_replay().unwrap());
            let final_cp = create_game_checkpoint(&resumed).unwrap();
            assert_eq!(
                resume_game_checkpoint(&final_cp)
                    .unwrap()
                    .replay_value()
                    .unwrap(),
                resumed.replay_value().unwrap()
            );
        }
    }
}

#[test]
fn intermediate_order_checkpoint_retains_sources_and_resumes_ranged_step() {
    let mut session = pending("north", 2, true);
    act(&mut session, |a| {
        a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == 0
    });
    let state = session.replay_value().unwrap();
    assert_eq!(state["state"]["phase"], "damage-order");
    assert_eq!(state["state"]["pendingDamageOrder"]["amount"], 3);
    assert_eq!(
        state["state"]["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let mut resumed = resume_game_checkpoint(
        &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
    )
    .unwrap();
    let receipt = act(&mut resumed, |a| {
        a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == 1
    });
    assert_eq!(
        receipt,
        act(&mut session, |a| a["kind"] == "choose-damage-modifier"
            && a["modifierIndex"] == 1)
    );
    let state = resumed.replay_value().unwrap();
    assert_eq!(state["state"]["players"]["south"]["avatar"]["life"], 13);
    assert_eq!(
        state["state"]["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(state["state"]["phase"], "ranged-step");
    act(&mut resumed, |a| {
        a["kind"] == "resolve-ranged-step" && a["choice"] == "decline"
    });
    assert!(resumed.verify_replay().unwrap());
}
