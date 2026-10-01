//! Public Session proof for Genesis effect strikes entering shared damage ordering.

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

fn manifest(first: &str) -> String {
    let thresholds = json!({"air":0,"earth":0,"fire":0,"water":0});
    let deck = json!({"avatar":"avatar", "atlas": vec!["site"; 8], "spellbook":vec!["genesis"; 8]});
    let mut value = json!({
        "schemaVersion":1,"engineVersion":"sorcery-core-v1","seed":91,"firstSeat":first,
        "authority":{"mode":"synthetic","revisionId":"effect-damage-order-test", "contentHash":identity_hash(&json!({"fixture":"effect-damage-order-test"})).unwrap()},
        "cards": {
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":{"cardType":"site","elements":["earth"]},
            "genesis":{"cardType":"minion","attack":2,"defense":3,"manaCost":0,"summonToAnySite":true,"genesisStrikeEachEnemyHere":true,"thresholds":thresholds,"entersCarrying":["bonus","double"]},
            "bonus":{"cardType":"artifact","token":true,"manaCost":null,"thresholds":thresholds,"bearerUnitStrike":{"damageBonus":1,"destroyAfterStrike":true}},
            "double":{"cardType":"artifact","token":true,"manaCost":null,"thresholds":thresholds,"nearbyStrikesAgainstUnitsDealDoubleDamage":true},
        },
        "decks":{"north":deck,"south":deck},
    });
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn manifest_with_strike_healing(first: &str) -> String {
    (91..4096)
        .map(|seed| {
            let mut value: Value = serde_json::from_str(&manifest(first)).unwrap();
            value["seed"] = json!(seed);
            value["cards"]["genesis"]["healsControllerForStrikeDamage"] = json!(true);
            value["cards"]["wound"] = json!({
                "cardType":"magic",
                "damageTargetUnit":1,
                "manaCost":0,
                "payLifeAsAdditionalCost":1,
                "thresholds":{"air":0,"earth":0,"fire":0,"water":0},
            });
            value["decks"][first]["spellbook"] = json!([
                "genesis", "genesis", "genesis", "genesis", "genesis", "genesis", "genesis",
                "wound"
            ]);
            value.as_object_mut().unwrap().remove("manifestId");
            value["manifestId"] = json!(identity_hash(&value).unwrap());
            canonical_json(&value).unwrap()
        })
        .find(|encoded| {
            let session = Session::new(encoded).unwrap();
            let replay = session.replay_value().unwrap();
            let hand = replay["state"]["players"][first]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            hand.iter().any(|card| card["cardId"] == "genesis")
                && hand.iter().any(|card| card["cardId"] == "wound")
        })
        .expect("seed with Genesis and life-cost Magic in opening hand")
}

fn pending(first: &str) -> Session {
    pending_from_manifest(first, &manifest(first))
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
    let own_home = if first == "north" { "C4" } else { "C1" };
    let enemy_home = if first == "north" { "C1" } else { "C4" };
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == own_home
    });
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == enemy_home
    });
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    let manifest: Value = serde_json::from_str(encoded).unwrap();
    if manifest["cards"]["genesis"]["healsControllerForStrikeDamage"] == true {
        let target_seat = if first == "north" { "south" } else { "north" };
        act(&mut session, |a| {
            a["kind"] == "cast-magic"
                && a["cardId"] == "wound"
                && a["target"]["kind"] == "avatar"
                && a["target"]["seat"] == target_seat
        });
        assert_eq!(
            session.replay_value().unwrap()["state"]["players"][first]["avatar"]["life"],
            19
        );
    }
    act(&mut session, |a| {
        a["kind"] == "summon-minion" && a["cardId"] == "genesis" && a["cell"] == enemy_home
    });
    assert_eq!(
        session.replay_value().unwrap()["state"]["phase"],
        "damage-order"
    );
    session
}

#[test]
fn effect_damage_order_checkpoint_retains_strike_healing_fact() {
    let ordinary = pending("north").replay_value().unwrap();
    assert!(
        ordinary["state"]["pendingDamageOrder"]["continuation"]["strike"]
            .get("healsControllerForStrikeDamage")
            .is_none()
    );
    let original = pending_from_manifest("north", &manifest_with_strike_healing("north"));
    let before = original.replay_value().unwrap();
    assert_eq!(
        before["state"]["pendingDamageOrder"]["continuation"]["strike"]["healsControllerForStrikeDamage"],
        true
    );
    assert_eq!(before["state"]["players"]["north"]["avatar"]["life"], 19);
    assert!(
        !original
            .transcript()
            .iter()
            .flat_map(|receipt| &receipt.events)
            .any(|event| { event.event_type == "avatar-healed" })
    );
    let checkpoint = create_game_checkpoint(&original).unwrap();
    let mut resumed = resume_game_checkpoint(
        &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(resumed.replay_value().unwrap(), before);
    let mut branch = original.clone();
    let index = resumed
        .legal_actions()
        .unwrap()
        .into_iter()
        .find_map(|action| {
            (action.descriptor["kind"] == "choose-damage-modifier")
                .then(|| action.descriptor["modifierIndex"].as_u64().unwrap())
        })
        .unwrap();
    let resumed_receipt = act(&mut resumed, |a| {
        a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
    });
    let healing: Vec<_> = resumed_receipt
        .events
        .iter()
        .filter(|event| event.event_type == "avatar-healed")
        .collect();
    let allocated = resumed_receipt
        .events
        .iter()
        .find(|event| event.event_type == "strike-damage-allocated")
        .expect("effect strike allocation");
    assert_eq!(healing.len(), 1);
    assert_eq!(healing[0].payload["seat"], "north");
    assert_eq!(
        healing[0].payload["sourceInstanceId"],
        before["state"]["realm"]["units"][0]["instanceId"]
    );
    assert_eq!(
        healing[0].payload["attemptedAmount"],
        allocated.payload["amount"]
    );
    assert_eq!(healing[0].payload["amount"], 1);
    assert_eq!(
        resumed.replay_value().unwrap()["state"]["players"]["north"]["avatar"]["life"],
        20
    );
    let branch_receipt = act(&mut branch, |a| {
        a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
    });
    assert_eq!(resumed_receipt, branch_receipt);
    assert_eq!(
        resumed.replay_value().unwrap(),
        branch.replay_value().unwrap()
    );
    assert!(resumed.verify_replay().unwrap());
}

fn leap_pending(first: &str) -> Session {
    let mut value: Value = serde_json::from_str(&manifest(first)).unwrap();
    let mut ally = value["cards"]["genesis"].clone();
    ally.as_object_mut()
        .unwrap()
        .remove("genesisStrikeEachEnemyHere");
    ally["attack"] = json!(2);
    value["cards"]["ally"] = ally;
    value["cards"].as_object_mut().unwrap().remove("genesis");
    value["cards"]["leap"] = json!({
        "cardType":"magic", "leapAttackAlly":true, "manaCost":0,
        "thresholds":{"air":0,"earth":0,"fire":0,"water":0}
    });
    value["cards"]["enemy"] = json!({
        "cardType":"minion", "attack":1, "defense":3, "manaCost":0,
        "summonToAnySite":true, "thresholds":{"air":0,"earth":0,"fire":0,"water":0}
    });
    value["decks"][first]["spellbook"] = json!(vec![
        "ally", "leap", "ally", "leap", "ally", "leap", "ally", "leap",
    ]);
    value["decks"][if first == "north" { "south" } else { "north" }]["spellbook"] =
        json!(vec!["enemy"; 8]);
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
    let own_home = if first == "north" { "C4" } else { "C1" };
    let enemy_home = if first == "north" { "C1" } else { "C4" };
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == own_home
    });
    act(&mut session, |a| {
        a["kind"] == "summon-minion" && a["cardId"] == "ally"
    });
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    act(&mut session, |a| {
        a["kind"] == "play-site" && a["cell"] == enemy_home
    });
    act(&mut session, |a| {
        a["kind"] == "summon-minion" && a["cardId"] == "enemy" && a["cell"] == own_home
    });
    act(&mut session, |a| a["kind"] == "end-turn");
    act(&mut session, |a| {
        a["kind"] == "draw" && a["zone"] == "atlas"
    });
    let ally_id = session.replay_value().unwrap()["state"]["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["cardId"] == "ally")
        .unwrap()["instanceId"]
        .clone();
    act(&mut session, |a| {
        a["kind"] == "cast-magic"
            && a["cardId"] == "leap"
            && a["ally"]["instanceId"] == ally_id
            && a["allyDestination"]["cell"] == own_home
    });
    assert_eq!(
        session.replay_value().unwrap()["state"]["phase"],
        "damage-order"
    );
    session
}

#[test]
fn genesis_effect_strike_damage_order_replays_and_consumes_for_both_seats() {
    for first in ["north", "south"] {
        let original = pending(first);
        let state = original.replay_value().unwrap();
        assert_eq!(state["state"]["pendingDamageOrder"]["amount"], 2);
        let checkpoint = create_game_checkpoint(&original).unwrap();
        let mut resumed = resume_game_checkpoint(
            &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
        )
        .unwrap();
        let mut branch = original.clone();
        let index = branch
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "choose-damage-modifier"
                    && branch.replay_value().unwrap()["state"]["pendingDamageOrder"]["remaining"]
                        [usize::try_from(action.descriptor["modifierIndex"].as_u64().unwrap())
                            .unwrap()]["operation"]
                        == "add"
            })
            .and_then(|action| action.descriptor["modifierIndex"].as_u64())
            .expect("add modifier choice");
        let receipt = act(&mut branch, |a| {
            a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
        });
        assert_eq!(
            receipt,
            act(&mut resumed, |a| {
                a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
            })
        );
        assert_eq!(
            branch.replay_value().unwrap(),
            resumed.replay_value().unwrap()
        );
        let state = resumed.replay_value().unwrap();
        let enemy = if first == "north" { "south" } else { "north" };
        assert_eq!(state["state"]["players"][enemy]["avatar"]["life"], 14);
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "artifact-consumed-after-strike")
                .count(),
            1
        );
        assert!(resumed.verify_replay().unwrap());
    }
}

#[test]
fn leap_effect_strike_damage_order_delays_magic_resolution_without_retaliation() {
    for first in ["north", "south"] {
        let original = leap_pending(first);
        let checkpoint = create_game_checkpoint(&original).unwrap();
        let mut resumed = resume_game_checkpoint(
            &parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap(),
        )
        .unwrap();
        let before = resumed.replay_value().unwrap();
        assert_eq!(before["state"]["phase"], "damage-order");
        assert!(
            before["transcript"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| {
                    entry["events"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|event| event["eventType"] != "magic-resolved")
                })
        );
        let index = resumed
            .legal_actions()
            .unwrap()
            .into_iter()
            .find_map(|action| {
                (action.descriptor["kind"] == "choose-damage-modifier")
                    .then(|| action.descriptor["modifierIndex"].as_u64().unwrap())
            })
            .unwrap();
        let receipt = act(&mut resumed, |a| {
            a["kind"] == "choose-damage-modifier" && a["modifierIndex"] == index
        });
        let after = resumed.replay_value().unwrap();
        assert_eq!(
            after["state"]["players"][if first == "north" { "south" } else { "north" }]["avatar"]["life"],
            20
        );
        let units = after["state"]["realm"]["units"].as_array().unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0]["cardId"], "ally");
        assert_eq!(units[0]["damage"], 0);
        assert_eq!(
            receipt
                .events
                .iter()
                .find(|event| event.event_type == "strike-damage-allocated")
                .unwrap()
                .payload["amount"],
            6
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "magic-resolved")
                .count(),
            1
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "artifact-consumed-after-strike")
                .count(),
            1
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "strike-damage-allocated")
                .count(),
            1
        );
        assert!(resumed.verify_replay().unwrap());
    }
}
