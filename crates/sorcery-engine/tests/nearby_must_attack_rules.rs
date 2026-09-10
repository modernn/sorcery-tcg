//! Direct proofs for nearby mandatory attacks and nearby doubled unit strikes
//! (RULE-CATALOG-0247–0251).
//!
//! Official cards such as Mask of Mayhem require every nearby minion that can
//! attack to do so, then double nearby strikes against units. The two sentences
//! are separate facts that may compose on one Artifact.

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyMinionsMustAttackIfAble": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "nearby-must-attack-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-nearby-must-attack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({ "charge": true })),
            "south-avatar": avatar(),
            "south-mask": mask(),
            "south-minion": minion(json!({
                "attack": 1,
                "defense": 1,
                "summonToAnySite": true,
            })),
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
                "spellbook": [
                    "south-mask",
                    "south-minion",
                    "south-minion",
                    "south-mask",
                    "south-minion",
                    "south-minion",
                ],
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

fn opening_manifest() -> String {
    (1..=4096)
        .map(manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("mask candidate"));
            let hand = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening spellbook");
            hand.iter().any(|card| card["cardId"] == "south-mask")
                && hand.iter().any(|card| card["cardId"] == "south-minion")
        })
        .expect("bounded seed opening with a Mask and a south minion")
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

fn after_north_summons(south_cell: &str) -> Session {
    let mut session = Session::new(&opening_manifest()).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == south_cell
    });
    let bearer_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion")
        .expect("south minion")["instanceId"]
        .clone();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-mask"
            && descriptor["bearer"]["instanceId"] == bearer_id
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    session
}

#[test]
fn rule_catalog_0247_nearby_minions_must_attack_if_able_before_optional_actions() {
    let mut session = after_north_summons("C4");
    assert_eq!(state(&session)["phase"], "main");
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("north minion")["instanceId"]
        .clone();
    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion")
        .expect("south minion")["instanceId"]
        .clone();
    let legal = session.legal_actions().expect("mandatory attacks");
    assert!(!legal.is_empty());
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == source_id
        }),
        "a nearby vanilla Charge minion that can attack must do so before optional actions"
    );
    assert!(legal.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack" && action.descriptor["to"]["cell"] == "C4"
    }));
    let checkpoint = session.clone();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(&session)["phase"] == "movement" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    assert_eq!(state(&session)["phase"], "attack");
    let attack_actions = session.legal_actions().expect("attack actions");
    assert!(
        attack_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "decline-attack")
    );
    assert_eq!(
        attack_actions
            .iter()
            .filter(|action| action.descriptor["kind"] == "declare-attack")
            .count(),
        1
    );
    assert_eq!(
        attack_actions[0].descriptor["target"]["instanceId"],
        target_id
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(&session)["phase"] == "intercept" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    assert_eq!(state(&session)["phase"], "main");
    let mut resumed = checkpoint;
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(&resumed)["phase"] == "movement" {
        accept_where(&mut resumed, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["instanceId"] == target_id
    });
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(&resumed)["phase"] == "intercept" {
        accept_where(&mut resumed, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    assert_eq!(
        resumed.replay_value().expect("resumed value"),
        session.replay_value().expect("session value")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0248_nearby_must_attack_does_not_constrain_when_not_nearby() {
    let mut session = after_north_summons("C1");
    assert_eq!(state(&session)["phase"], "main");
    let legal = session.legal_actions().expect("ordinary main actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(state(&session)["phase"], "draw");
    assert_exact_replay(&session);
}

fn after_walk_away_summons() -> Session {
    let mut session = Session::new(&opening_manifest()).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-mask"
            && descriptor["cell"] == "C2"
            && descriptor["bearer"].is_null()
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C3"
    });
    session
}

#[test]
fn rule_catalog_0249_nearby_must_attack_may_decline_after_leaving_nearby() {
    let mut session = after_walk_away_summons();
    assert_eq!(state(&session)["phase"], "main");
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("north minion")["instanceId"]
        .clone();
    let legal = session.legal_actions().expect("mandatory attacks");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == source_id
        }),
        "a nearby minion that can attack must take Move and Attack"
    );
    assert!(legal.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack" && action.descriptor["to"]["cell"] == "C4"
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(&session)["phase"] == "movement" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    assert_eq!(state(&session)["phase"], "attack");
    let attack_actions = session.legal_actions().expect("attack actions");
    assert!(
        attack_actions
            .iter()
            .any(|action| action.descriptor["kind"] == "decline-attack"),
        "leaving nearby the Mask makes the later attack optional"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&session)["phase"], "main");
    assert_exact_replay(&session);
}

fn double_mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyMinionsMustAttackIfAble": true,
        "nearbyStrikesAgainstUnitsDealDoubleDamage": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn double_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "nearby-double-strike-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-nearby-double-strike-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "attack": 1,
                "charge": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-mask": double_mask(),
            "south-minion": minion(json!({
                "attack": 1,
                "defense": 2,
                "summonToAnySite": true,
            })),
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
                "spellbook": [
                    "south-mask",
                    "south-minion",
                    "south-minion",
                    "south-mask",
                    "south-minion",
                    "south-minion",
                ],
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

fn double_opening_manifest() -> String {
    (1..=4096)
        .map(double_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("double candidate"));
            let hand = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening spellbook");
            hand.iter().any(|card| card["cardId"] == "south-mask")
                && hand.iter().any(|card| card["cardId"] == "south-minion")
        })
        .expect("bounded seed opening with a composed Mask and a south minion")
}

fn after_double_summons(carry_mask: bool) -> Session {
    let mut session = Session::new(&double_opening_manifest()).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    if carry_mask {
        let bearer_id = state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["cardId"] == "south-minion")
            .expect("south minion")["instanceId"]
            .clone();
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["bearer"]["instanceId"] == bearer_id
        });
    } else {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["cell"] == "C1"
                && descriptor["bearer"].is_null()
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    session
}

fn fight_c4(session: &mut Session, source_id: &Value, target_id: &Value) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == *source_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(session)["phase"] == "movement" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == *target_id
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
}

#[test]
fn rule_catalog_0250_nearby_strikes_against_units_deal_double_damage() {
    let mut session = after_double_summons(true);
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("north minion")["instanceId"]
        .clone();
    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion")
        .expect("south minion")["instanceId"]
        .clone();
    let legal = session
        .legal_actions()
        .expect("composed Mask forces the attack");
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == source_id
    }));
    fight_c4(&mut session, &source_id, &target_id);
    assert_eq!(state(&session)["phase"], "main");
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == source_id)
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id),
        "a 1-power nearby strike must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0251_nearby_double_damage_does_not_apply_when_the_struck_unit_is_not_nearby() {
    let mut session = after_double_summons(false);
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("north minion")["instanceId"]
        .clone();
    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion")
        .expect("south minion")["instanceId"]
        .clone();
    let legal = session.legal_actions().expect("ordinary main actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    fight_c4(&mut session, &source_id, &target_id);
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("south minion survives an undoubled 1-power strike");
    assert_eq!(target["damage"], 1);
    assert_exact_replay(&session);
}
