use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct FightResult {
    attacker_instance_id: String,
    fight: Receipt,
    session: Session,
    target_instance_id: String,
}

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

fn minion(attack: u8, defense: u8) -> Value {
    json!({
        "attack": attack,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(seed: u32, source_power: u8, disabled_target: bool) -> String {
    let mut attacker = minion(source_power, 5);
    attacker["charge"] = json!(true);
    attacker["summonToAnySite"] = json!(true);
    let mut target = minion(1, 10);
    target["preventsDamageFromUnitsWithPowerAtLeast"] = json!(4);
    if disabled_target {
        target["genesisDisableSelfUntilDamaged"] = json!(true);
    }
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "damage-prevention-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-damage-prevention-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-minion": attacker,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": target,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 5],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 5],
            },
            "south": {
                "atlas": vec!["south-site"; 5],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 5],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] =
        json!(identity_hash(&manifest).expect("canonical synthetic manifest identity"));
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn unit_state(session: &Session, instance_id: &str) -> Value {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
        .clone()
}

fn event_values(receipt: &Receipt) -> Vec<Value> {
    receipt
        .events
        .iter()
        .map(|event| json!({ "payload": event.payload, "type": event.event_type }))
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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

fn resolve_fight(seed: u32, source_power: u8, disabled_target: bool) -> FightResult {
    let manifest = scenario_manifest(seed, source_power, disabled_target);
    let mut session = Session::new(&manifest).expect("valid damage-prevention scenario");
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
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned_target, summon_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "south-minion"
    });
    let target_instance_id = summoned_target["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    let expected_summon_events = if disabled_target {
        vec!["minion-summoned", "minion-disabled"]
    } else {
        vec!["minion-summoned"]
    };
    assert_eq!(
        summon_receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        expected_summon_events
    );
    assert_eq!(
        unit_state(&session, &target_instance_id)["disabledUntilDamaged"],
        if disabled_target {
            json!(true)
        } else {
            Value::Null
        }
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (summoned_attacker, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C1"
    });
    let attacker_instance_id = summoned_attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    FightResult {
        attacker_instance_id,
        fight,
        session,
        target_instance_id,
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct rule proof compares all threshold and disabled cases"
)]
fn active_minion_should_prevent_damage_from_unit_at_current_power_threshold() {
    let prevented = resolve_fight(170, 4, false);
    assert_eq!(
        event_values(&prevented.fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": prevented.attacker_instance_id,
                    "combatantInstanceIds": [prevented.target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 4,
                    "strikerInstanceId": prevented.attacker_instance_id,
                    "targetInstanceId": prevented.target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": prevented.attacker_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "accumulated": 0,
                    "amount": 0,
                    "attemptedAmount": 4,
                    "direct": true,
                    "instanceId": prevented.target_instance_id,
                    "prevented": true,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
        ]
    );
    assert_eq!(
        unit_state(&prevented.session, &prevented.target_instance_id)["damage"],
        0
    );
    assert!(prevented.fight.random_draws.is_empty());
    assert_exact_replay(&prevented.session);

    let below_threshold = resolve_fight(172, 3, false);
    let target_damage = below_threshold
        .fight
        .events
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt"
                && event.payload["instanceId"] == below_threshold.target_instance_id
        })
        .expect("target damage event");
    assert_eq!(
        target_damage.payload,
        json!({
            "accumulated": 3,
            "amount": 3,
            "direct": true,
            "instanceId": below_threshold.target_instance_id,
            "seat": "south",
        })
    );
    assert_eq!(
        unit_state(
            &below_threshold.session,
            &below_threshold.target_instance_id,
        )["damage"],
        3
    );
    assert!(below_threshold.fight.random_draws.is_empty());
    assert_exact_replay(&below_threshold.session);

    let disabled = resolve_fight(173, 4, true);
    assert_eq!(
        event_values(&disabled.fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": disabled.attacker_instance_id,
                    "combatantInstanceIds": [disabled.target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 4,
                    "strikerInstanceId": disabled.attacker_instance_id,
                    "targetInstanceId": disabled.target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 4,
                    "amount": 4,
                    "direct": true,
                    "instanceId": disabled.target_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "instanceId": disabled.target_instance_id,
                    "seat": "south",
                },
                "type": "minion-awakened",
            }),
        ]
    );
    let awakened_target = unit_state(&disabled.session, &disabled.target_instance_id);
    assert_eq!(awakened_target["damage"], 4);
    assert_eq!(awakened_target["disabledUntilDamaged"], Value::Null);
    assert_eq!(
        unit_state(&disabled.session, &disabled.attacker_instance_id)["damage"],
        0
    );
    assert!(disabled.fight.random_draws.is_empty());
    assert_exact_replay(&disabled.session);
}
