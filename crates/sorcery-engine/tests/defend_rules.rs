use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn minion(movement_bonus: u8, moves_only_sideways: bool) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 1,
        "movementBonus": movement_bonus,
        "movesOnlySideways": moves_only_sideways,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn attacker(attack: u8, defense: u8) -> Value {
    json!({
        "attack": attack,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(seed: u32, defender: &Value, attacker: &Value) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "defend-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-defend-rules-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-defender": defender,
            "north-site": site,
            "south-attacker": attacker,
            "south-avatar": avatar,
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": ["north-site", "north-site", "north-site", "north-site"],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-defender", "north-defender", "north-defender", "north-defender",
                    "north-defender", "north-defender", "north-defender", "north-defender"
                ],
            },
            "south": {
                "atlas": ["south-site", "south-site", "south-site", "south-site"],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-attacker", "south-attacker", "south-attacker", "south-attacker",
                    "south-attacker", "south-attacker", "south-attacker", "south-attacker"
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
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

fn exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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

struct AttackSetup {
    attacker_id: String,
    defender_id: String,
    extra_defender_ids: Vec<String>,
    session: Session,
    target_id: Option<String>,
}

#[expect(
    clippy::too_many_lines,
    reason = "shared scenario setup keeps exact public turn sequencing visible"
)]
fn declared_attack(
    seed: u32,
    defender: &Value,
    attacker: &Value,
    target_minion: bool,
    extra_defenders: usize,
) -> AttackSetup {
    let manifest = scenario_manifest(seed, defender, attacker);
    let mut session = Session::new(&manifest).expect("valid Defend scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-defender"
            && descriptor["cell"] == "C4"
    });
    let defender_id = summon["cardInstanceId"]
        .as_str()
        .expect("defender identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-attacker"
            && descriptor["cell"] == "C1"
    });
    let attacker_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let mut extra_defender_ids = Vec::with_capacity(extra_defenders);
    for _ in 0..extra_defenders {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-defender"
                && descriptor["cell"] == "C3"
        });
        extra_defender_ids.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("extra defender identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let target_id = target_minion.then(|| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-defender"
                && descriptor["cell"] == "C2"
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("target identity")
            .to_owned()
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["path"]
                == json!([
                    { "cell": "C1", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && if let Some(target_id) = &target_id {
                descriptor["target"]["kind"] == "minion"
                    && descriptor["target"]["instanceId"] == target_id.as_str()
            } else {
                descriptor["target"]["kind"] == "site"
            }
    });

    AttackSetup {
        attacker_id,
        defender_id,
        extra_defender_ids,
        session,
        target_id,
    }
}

#[test]
fn rule_catalog_0771_sideways_restriction_excludes_vertical_defend_paths() {
    let AttackSetup {
        defender_id,
        session,
        ..
    } = declared_attack(128, &minion(1, true), &attacker(1, 1), false, 0);
    let defender_actions: Vec<Value> = session
        .legal_actions()
        .expect("Defend actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "defend"
                && action.descriptor["unitInstanceId"] == defender_id
        })
        .map(|action| action.descriptor)
        .collect();

    assert_eq!(defender_actions, Vec::<Value>::new());
    exact_replay(&session);
}

#[test]
fn rule_catalog_0777_movement_bonus_one_issues_exact_two_step_defend_path() {
    let AttackSetup {
        defender_id,
        mut session,
        ..
    } = declared_attack(54, &minion(1, false), &attacker(1, 1), false, 0);
    let expected_path = json!([
        { "cell": "C4", "region": "surface" },
        { "cell": "C3", "region": "surface" },
        { "cell": "C2", "region": "surface" },
    ]);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == defender_id
            && descriptor["path"] == expected_path
    });
    let state = &session.replay_value().expect("authoritative state")["state"];
    let defender = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == defender_id)
        .expect("moved defender");

    assert_eq!(
        (
            descriptor,
            receipt
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .collect::<Vec<_>>(),
            defender["location"].clone(),
            defender["tapped"].clone(),
            state["phase"].clone(),
        ),
        (
            json!({
                "from": { "cell": "C4", "region": "surface" },
                "kind": "defend",
                "path": expected_path,
                "to": { "cell": "C2", "region": "surface" },
                "unitInstanceId": defender_id,
            }),
            vec!["defender-joined", "original-target-removed"],
            json!("C2"),
            json!(true),
            json!("defend"),
        )
    );
    exact_replay(&session);
}

#[test]
fn rule_catalog_0788_defend_moves_then_resolves_simultaneous_split_damage() {
    let AttackSetup {
        attacker_id,
        defender_id,
        mut session,
        target_id,
        ..
    } = declared_attack(53, &minion(1, false), &attacker(1, 1), true, 0);
    let target_id = target_id.expect("original minion target");
    let (_, joined) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == defender_id
            && descriptor["to"]["cell"] == "C2"
    });
    let joined_state = session.replay_value().expect("joined state")["state"].clone();
    let (_, closed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let allocate_state = session.replay_value().expect("allocation state")["state"].clone();
    let (first_descriptor, first_allocation) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let (final_allocation, final_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 1
    });
    let damaged_id = final_allocation["targetInstanceId"]
        .as_str()
        .expect("allocated target");
    let state = &session.replay_value().expect("authoritative state")["state"];
    let living_ids: Vec<&str> = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter_map(|unit| unit["instanceId"].as_str())
        .collect();
    let event_types: Vec<&str> = first_allocation
        .events
        .iter()
        .chain(&final_receipt.events)
        .map(|event| event.event_type.as_str())
        .collect();
    let allocation_amounts: Vec<u64> = first_allocation
        .events
        .iter()
        .chain(&final_receipt.events)
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| event.payload["amount"].as_u64().expect("allocation amount"))
        .collect();

    assert!(
        state["phase"] == "main"
            && joined_state["phase"] == "defend"
            && joined_state["realm"]["units"]
                .as_array()
                .expect("joined units")
                .iter()
                .any(|unit| {
                    unit["instanceId"] == defender_id
                        && unit["location"] == "C2"
                        && unit["tapped"] == true
                })
            && joined
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .eq(["defender-joined"])
            && allocate_state["phase"] == "allocate"
            && allocate_state["decisionSeat"] == "south"
            && closed
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .eq(["defend-window-closed", "fight-started"])
            && first_descriptor["amount"] == 0
            && final_allocation["amount"] == 1
            && allocation_amounts == [0, 1]
            && !living_ids.contains(&attacker_id.as_str())
            && !living_ids.contains(&damaged_id)
            && living_ids.contains(&target_id.as_str())
                != living_ids.contains(&defender_id.as_str())
            && event_types
                .iter()
                .filter(|event_type| **event_type == "damage-dealt")
                .count()
                == 3
            && event_types
                .iter()
                .filter(|event_type| **event_type == "minion-died")
                .count()
                == 2,
        "Defend must stage one split allocation and resolve all three strikes simultaneously: living={living_ids:?} events={event_types:?} damaged={damaged_id} state={state}"
    );
    exact_replay(&session);
}

#[test]
fn rule_catalog_0795_takes_less_damage_prevents_each_simultaneous_lethal_source() {
    let mut lethal_defender = minion(1, false);
    lethal_defender["lethal"] = json!(true);
    let mut resilient_attacker = attacker(2, 2);
    resilient_attacker["takesLessDamage"] = json!(1);
    let AttackSetup {
        attacker_id,
        defender_id,
        mut session,
        target_id,
        ..
    } = declared_attack(142, &lethal_defender, &resilient_attacker, true, 0);
    let target_id = target_id.expect("original minion target");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == defender_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 1
    });
    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 1
    });
    let state = &session.replay_value().expect("authoritative state")["state"];
    let attacker = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == attacker_id)
        .expect("surviving attacker");
    let damage = fought.events.iter().find(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == attacker_id
    });

    assert!(
        attacker["damage"] == 0
            && state["realm"]["units"]
                .as_array()
                .expect("realm units")
                .iter()
                .all(|unit| {
                    unit["instanceId"] != defender_id && unit["instanceId"] != target_id
                })
            && damage.is_some_and(|event| {
                event.payload["amount"] == 0
                    && event.payload["attemptedAmount"] == 2
                    && event.payload["prevented"] == true
            })
            && fought
                .events
                .iter()
                .filter(|event| event.event_type == "minion-died")
                .count()
                == 2,
        "each one-damage Lethal source must be reduced before simultaneous aggregation"
    );
    exact_replay(&session);
}

#[test]
fn rule_catalog_0750_joining_defend_preserves_stealth_until_fight_interaction() {
    let mut stealthed_defender = minion(1, false);
    stealthed_defender["stealth"] = json!(true);
    let AttackSetup {
        defender_id,
        mut session,
        ..
    } = declared_attack(207, &stealthed_defender, &attacker(1, 1), false, 0);
    let before = session.replay_value().expect("pre-Defend state")["state"].clone();
    let (_, joined) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == defender_id
    });
    let joined_state = session.replay_value().expect("joined state")["state"].clone();
    let defender_before = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == defender_id)
        .expect("Defend candidate");
    let joined_defender = joined_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == defender_id)
        .expect("joined defender");

    assert!(
        defender_before["stealthed"] == true
            && defender_before.get("lastInteractedTurn").is_none()
            && joined_defender["stealthed"] == true
            && joined_defender.get("lastInteractedTurn").is_none()
            && joined
                .events
                .iter()
                .all(|event| event.event_type != "stealth-lost"),
        "joining Defend is movement, not the fight interaction"
    );

    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert!(fought.events.iter().any(|event| {
        event.event_type == "stealth-lost" && event.payload["instanceId"] == defender_id
    }));
    exact_replay(&session);
}

#[test]
fn rule_catalog_0764_simultaneous_return_damage_above_u8_records_exact_total() {
    let mut powerful_defender = minion(1, false);
    powerful_defender["attack"] = json!(100);
    powerful_defender["defense"] = json!(1);
    let AttackSetup {
        attacker_id,
        defender_id,
        extra_defender_ids,
        mut session,
        target_id,
    } = declared_attack(208, &powerful_defender, &attacker(0, 100), true, 1);
    let target_id = target_id.expect("original minion target");
    let extra_defender_id = extra_defender_ids
        .first()
        .expect("extra defender identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == defender_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == extra_defender_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let state = &session.replay_value().expect("authoritative state")["state"];
    let exact_damage = fought.events.iter().find(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == attacker_id
    });
    let living_ids: Vec<&str> = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter_map(|unit| unit["instanceId"].as_str())
        .collect();

    assert!(
        exact_damage.is_some_and(|event| {
            event.payload["amount"] == 300 && event.payload["accumulated"] == 300
        }) && !living_ids.contains(&attacker_id.as_str())
            && living_ids.contains(&defender_id.as_str())
            && living_ids.contains(&extra_defender_id.as_str())
            && living_ids.contains(&target_id.as_str()),
        "three legal 100-power return strikes must aggregate to 300 without u8 overflow"
    );
    exact_replay(&session);
}

#[test]
fn rule_catalog_0907_defend_simultaneous_lethal_kills_tougher_attacker() {
    let mut lethal_defender = minion(1, false);
    lethal_defender["lethal"] = json!(true);
    let tough_attacker = attacker(1, 2);
    let AttackSetup {
        attacker_id,
        defender_id,
        mut session,
        ..
    } = declared_attack(143, &lethal_defender, &tough_attacker, false, 0);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == defender_id
            && descriptor["to"]["cell"] == "C2"
    });
    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    let state = &session.replay_value().expect("authoritative state")["state"];
    let living_ids: Vec<&str> = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter_map(|unit| unit["instanceId"].as_str())
        .collect();
    let attacker_damage = fought.events.iter().find(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == attacker_id
    });
    let defender_damage = fought.events.iter().find(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == defender_id
    });
    assert!(
        living_ids.is_empty()
            && attacker_damage.is_some_and(|event| {
                event.payload["amount"] == 1 && event.payload["accumulated"] == 1
            })
            && defender_damage.is_some_and(|event| event.payload["amount"] == 1)
            && fought.events.iter().any(|event| {
                event.event_type == "minion-died" && event.payload["instanceId"] == attacker_id
            })
            && fought.events.iter().any(|event| {
                event.event_type == "minion-died" && event.payload["instanceId"] == defender_id
            })
            && fought
                .events
                .iter()
                .filter(|event| event.event_type == "minion-died")
                .count()
                == 2,
        "one printed-Lethal return strike in a simultaneous Defend fight must destroy a 2-defense attacker from a single damage: living={living_ids:?} events={:?}",
        fought
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>()
    );
    exact_replay(&session);
}

#[test]
fn rule_catalog_0938_warded_attacker_absorbs_three_way_simultaneous_return_split() {
    let mut warded_attacker = attacker(0, 1);
    warded_attacker["ward"] = json!(true);
    let AttackSetup {
        attacker_id,
        defender_id,
        extra_defender_ids,
        mut session,
        target_id,
    } = declared_attack(209, &minion(1, false), &warded_attacker, true, 1);
    let target_id = target_id.expect("original minion target");
    let extra_defender_id = extra_defender_ids
        .first()
        .expect("extra defender identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == defender_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == extra_defender_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let (_, first_allocation) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let (_, second_allocation) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "allocate-strike" && descriptor["amount"] == 0
    });
    let state = &session.replay_value().expect("authoritative state")["state"];
    let attacker = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == attacker_id)
        .expect("surviving warded attacker");
    let living_ids: Vec<&str> = state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter_map(|unit| unit["instanceId"].as_str())
        .collect();
    let attacker_damage = fought.events.iter().find(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == attacker_id
    });
    let allocation_amounts: Vec<u64> = first_allocation
        .events
        .iter()
        .chain(&second_allocation.events)
        .chain(&fought.events)
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| event.payload["amount"].as_u64().expect("allocation amount"))
        .collect();
    let event_types: Vec<&str> = fought
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();

    assert!(
        state["phase"] == "main"
            && allocation_amounts == [0, 0, 0]
            && attacker["damage"] == 0
            && attacker["warded"] == false
            && attacker_damage.is_some_and(|event| {
                event.payload["amount"] == 0
                    && event.payload["attemptedAmount"] == 3
                    && event.payload["prevented"] == true
            })
            && fought.events.iter().any(|event| {
                event.event_type == "ward-broken" && event.payload["instanceId"] == attacker_id
            })
            && living_ids.contains(&attacker_id.as_str())
            && living_ids.contains(&defender_id.as_str())
            && living_ids.contains(&extra_defender_id.as_str())
            && living_ids.contains(&target_id.as_str())
            && event_types
                .iter()
                .filter(|event_type| **event_type == "damage-dealt")
                .count()
                == 4
            && fought
                .events
                .iter()
                .filter(|event| event.event_type == "minion-died")
                .count()
                == 0,
        "three-way Defend split must aggregate simultaneous return damage once against Ward: living={living_ids:?} attacker={attacker:?} events={event_types:?}"
    );
    exact_replay(&session);
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn offers_kind(session: &Session, kind: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == kind)
    })
}

fn try_end_and_draw_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn no_kind(session: &Session, kind: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .all(|action| action.descriptor["kind"] != kind)
}

fn deathrite_plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn defend_aura() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "movementBonus": 1,
        "otherNearbyAlliesPowerBonus": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tough_attacker() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_defend_manifest(seed: u32) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "defend-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-defend-deathrite-withheld-v1",
        },
        "cards": {
            "north-aura": defend_aura(),
            "north-avatar": avatar,
            "north-deathrite": deathrite_plain(),
            "north-site": site,
            "south-attacker": tough_attacker(),
            "south-avatar": avatar,
            "south-rain": rain_spell(),
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-deathrite",
                    "north-deathrite",
                    "north-aura",
                    "north-deathrite",
                    "north-aura",
                    "north-deathrite",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-attacker",
                    "south-attacker",
                    "south-rain",
                    "south-rain",
                    "south-rain",
                    "south-rain",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
}

struct PendingDeathriteDefendSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_defend(encoded: &str) -> Option<PendingDeathriteDefendSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-deathrite"
            && descriptor["cell"] == "C4"
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-deathrite"
            && descriptor["cell"] == "C4"
    })?;
    let aura = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cell"] == "C4"
    })?;
    let aura_id = aura.0["cardInstanceId"].as_str()?.to_owned();
    try_end_and_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let attacker = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-attacker"
            && descriptor["cell"] == "C1"
    })?;
    let attacker_id = attacker.0["cardInstanceId"].as_str()?.to_owned();
    try_end_and_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_end_and_draw_spellbook(&mut session)?;
    try_end_and_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_end_and_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "south-rain"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["path"]
                == json!([
                    { "cell": "C1", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    })?;
    if state(&session)["phase"] != "defend"
        || !offers_kind(&session, "defend")
        || !offers_kind(&session, "close-defend")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == aura_id
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteDefendSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_defend_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_defend_manifest)
        .find(|candidate| try_pending_deathrite_during_defend(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites mid-Defend window")
}

#[test]
fn rule_catalog_1135_defend_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_defend_seed_with(1135);
    let mut setup = try_pending_deathrite_during_defend(&encoded)
        .expect("complete Defend Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "north");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "defend");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == aura_id)
            .expect("joined aura")["location"],
        "C2"
    );
    assert!(no_kind(session, "defend"));
    assert!(no_kind(session, "close-defend"));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "defend");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_kind(session, "close-defend"));

    let (_, closed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert!(
        closed
            .events
            .iter()
            .any(|event| event.event_type == "defend-window-closed")
    );
    exact_replay(session);
}

#[test]
fn rule_catalog_1158_close_defend_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_defend_seed_with(1158);
    let mut setup = try_pending_deathrite_during_defend(&encoded)
        .expect("complete close-defend Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "north");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "defend");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == aura_id)
            .expect("joined aura")["location"],
        "C2"
    );
    assert!(no_kind(session, "close-defend"));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "defend");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_kind(session, "close-defend"));
    exact_replay(session);
}
