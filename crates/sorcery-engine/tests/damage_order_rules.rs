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
        .unwrap_or_else(|| {
            let state = session.replay_value().expect("state");
            panic!(
                "issued action phase={} seat={}",
                state["state"]["phase"], state["state"]["decisionSeat"]
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
    pending_from_manifest(first, &manifest(first, bonuses, step_after))
}

fn pending_from_manifest(first: &str, encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("manifest");
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
fn ranged_stealth_survives_replacement_order_then_breaks_after_damage() {
    for first in ["north", "south"] {
        let mut value: Value = serde_json::from_str(&manifest(first, 1, false)).unwrap();
        value["cards"]["shooter"]["stealth"] = json!(true);
        value.as_object_mut().unwrap().remove("manifestId");
        value["manifestId"] = json!(identity_hash(&value).unwrap());
        let original = pending_from_manifest(first, &canonical_json(&value).unwrap());
        let parent = original.replay_value().unwrap();
        let shooter = &parent["state"]["realm"]["units"][0];
        let shooter_id = shooter["instanceId"].clone();
        assert_eq!(shooter["stealthed"], true);
        assert_eq!(shooter["tapped"], true);
        assert_eq!(shooter["controller"], first);
        let checkpoint = parse_game_checkpoint(
            &serialize_game_checkpoint(&create_game_checkpoint(&original).unwrap()).unwrap(),
        )
        .unwrap();
        for (index, expected) in [(0, 6), (1, 5)] {
            let mut branch = original.clone();
            let mut resumed = resume_game_checkpoint(&checkpoint).unwrap();
            let receipt = act(&mut branch, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            });
            assert_eq!(
                receipt,
                act(&mut resumed, |a| a["kind"] == "choose-damage-modifier"
                    && a["modifierIndex"] == index)
            );
            let events = &receipt.events;
            let event_index = |name| {
                events
                    .iter()
                    .position(|event| event.event_type == name)
                    .unwrap()
            };
            assert!(event_index("avatar-life-lost") < event_index("stealth-lost"));
            assert!(event_index("artifact-consumed-after-strike") < event_index("stealth-lost"));
            let lost: Vec<_> = events
                .iter()
                .filter(|event| event.event_type == "stealth-lost")
                .collect();
            assert_eq!(lost.len(), 1);
            assert_eq!(lost[0].payload["instanceId"], shooter_id);
            assert_eq!(
                events[event_index("strike-damage-allocated")].payload["amount"],
                expected
            );
            let completed = branch.replay_value().unwrap();
            assert_eq!(completed["state"]["realm"]["units"][0]["stealthed"], false);
            assert_eq!(completed, resumed.replay_value().unwrap());
            assert_eq!(branch.transcript(), resumed.transcript());
            assert_eq!(
                branch.legal_actions().unwrap(),
                resumed.legal_actions().unwrap()
            );
            for seat in [Seat::North, Seat::South] {
                assert_eq!(
                    branch.public_view(seat).unwrap(),
                    resumed.public_view(seat).unwrap()
                );
            }
            assert!(branch.verify_replay().unwrap());
            assert_eq!(original.replay_value().unwrap(), parent);
        }
    }
}

fn fight_pending(first: &str) -> Session {
    let mut value: Value = serde_json::from_str(&manifest(first, 1, false)).unwrap();
    value["cards"]["shooter"]["ranged"] = json!(false);
    value["cards"]["shooter"]["entersCarrying"] = json!(["bonus", "double"]);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    let encoded = canonical_json(&value).unwrap();
    let mut session = Session::new(&encoded).expect("manifest");
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
    let enemy = if first == "north" { "south" } else { "north" };
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
                a["kind"] == "summon-minion" && a["cardId"] == "shooter" && a["cell"] == cell
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
    act(&mut session, |a| {
        a["kind"] == "move-and-attack" && a["to"]["cell"] == enemy_home
    });
    while session.replay_value().unwrap()["state"]["phase"] == "movement" {
        act(&mut session, |a| a["kind"] == "continue-basic-movement");
    }
    act(&mut session, |a| {
        a["kind"] == "declare-attack"
            && a["target"]["kind"] == "avatar"
            && a["target"]["seat"] == enemy
    });
    act(&mut session, |a| {
        a["kind"] == "close-defend" && a["originalTargetParticipates"] == true
    });
    if session.replay_value().unwrap()["state"]["phase"] == "intercept" {
        act(&mut session, |a| a["kind"] == "close-intercept");
    }
    assert_eq!(
        session.replay_value().unwrap()["state"]["phase"],
        "damage-order"
    );
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

fn finish_fight_damage_order(session: &mut Session) {
    while session.replay_value().unwrap()["state"]["phase"] == "damage-order" {
        let index = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| action.descriptor["kind"] == "choose-damage-modifier")
            .and_then(|action| action.descriptor["modifierIndex"].as_u64())
            .expect("issued damage modifier choice");
        act(session, |a| {
            a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
        });
    }
}

#[test]
fn fight_damage_order_checkpoint_replays_all_packets_for_both_seats() {
    for first in ["north", "south"] {
        let original = fight_pending(first);
        let state = original.replay_value().unwrap();
        assert!(
            state["state"]["pendingDamageOrder"]["damage"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
        let parallel = std::thread::scope(|scope| {
            (0..4)
                .map(|_| {
                    let mut branch = original.clone();
                    scope.spawn(move || {
                        finish_fight_damage_order(&mut branch);
                        assert!(branch.verify_replay().unwrap());
                        branch.replay_value().unwrap()
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(parallel.windows(2).all(|values| values[0] == values[1]));
        let checkpoint = create_game_checkpoint(&original).unwrap();
        let mut resumed = resume_game_checkpoint(
            &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
        )
        .unwrap();
        let mut branch = original.clone();
        while branch.replay_value().unwrap()["state"]["phase"] == "damage-order" {
            let index = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "choose-damage-modifier")
                .and_then(|action| action.descriptor["modifierIndex"].as_u64())
                .expect("issued damage modifier choice");
            let receipt = act(&mut branch, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            });
            let resumed_receipt = act(&mut resumed, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            });
            assert_eq!(receipt, resumed_receipt);
        }
        assert_eq!(
            resumed.replay_value().unwrap(),
            branch.replay_value().unwrap()
        );
        assert!(resumed.verify_replay().unwrap());
        let completed = resumed.replay_value().unwrap();
        assert!(completed["state"]["pendingDamageOrder"].is_null());
        assert_eq!(
            completed["state"]["players"][if first == "north" { "south" } else { "north" }]["avatar"]
                ["life"],
            14
        );
        let units = completed["state"]["realm"]["units"].as_array().unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0]["damage"], 2);
        let final_checkpoint = create_game_checkpoint(&resumed).unwrap();
        assert_eq!(
            resume_game_checkpoint(&final_checkpoint)
                .unwrap()
                .replay_value()
                .unwrap(),
            resumed.replay_value().unwrap()
        );
    }
}
