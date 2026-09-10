//! Direct proofs for mandatory unit attacks (RULE-CATALOG-0243–0246).
//!
//! Official cards such as Twinnax Berserker require a minion to attack a unit
//! whenever it can. Official cards such as the Green Knight require enemy
//! minions that can attack that unit to do so, before optional main-phase
//! actions.

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

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "must-attack-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-must-attack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "charge": true,
                "mustAttackAUnitIfAble": true,
            })),
            "south-avatar": avatar(),
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
                "spellbook": vec!["south-minion"; 6],
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

fn after_north_summons(seed: u32, south_cell: &str) -> Session {
    let mut session = Session::new(&manifest(seed)).expect("valid session");
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
#[expect(
    clippy::too_many_lines,
    reason = "one function proves the mandatory attack, Attack-phase filter, and replay"
)]
fn rule_catalog_0243_must_attack_a_unit_if_able_before_optional_actions() {
    let mut session = after_north_summons(243, "C4");
    assert_eq!(state(&session)["phase"], "main");
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
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
                && action.descriptor["to"]["cell"] == "C4"
        }),
        "optional main-phase actions must wait until the unit attack is taken"
    );
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
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == source_id)
    );
    assert!(
        !state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id)
    );
    let mut resumed = checkpoint;
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "move-and-attack" && descriptor["unitInstanceId"] == source_id
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
fn rule_catalog_0244_must_attack_does_not_constrain_when_no_unit_is_in_range() {
    let mut session = after_north_summons(244, "C1");
    assert_eq!(state(&session)["phase"], "main");
    let legal = session.legal_actions().expect("ordinary main actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "play-site")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(state(&session)["phase"], "draw");
    assert_exact_replay(&session);
}

fn forced_source_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "forced-attack-source" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-forced-attack-source-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "charge": true,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({
                "attack": 1,
                "defense": 1,
                "enemiesMustAttackThisIfAble": true,
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
                "spellbook": vec!["south-minion"; 6],
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

fn after_north_summons_forced(seed: u32, south_cell: &str) -> Session {
    let mut session = Session::new(&forced_source_manifest(seed)).expect("valid session");
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
fn rule_catalog_0245_enemies_must_attack_this_if_able_before_optional_actions() {
    let mut session = after_north_summons_forced(245, "C4");
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
        .expect("forced source")["instanceId"]
        .clone();
    let legal = session.legal_actions().expect("mandatory attacks");
    assert!(!legal.is_empty());
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == source_id
                && action.descriptor["to"]["cell"] == "C4"
        }),
        "a vanilla Charge minion must attack the forced source before optional actions"
    );
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
        descriptor["kind"] == "move-and-attack" && descriptor["unitInstanceId"] == source_id
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
fn rule_catalog_0246_enemies_must_attack_this_does_not_constrain_out_of_range() {
    let mut session = after_north_summons_forced(246, "C1");
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
