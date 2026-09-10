use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct AttackSetup {
    attacker_id: String,
    defender_id: Option<String>,
    session: Session,
    survivor_id: Option<String>,
    target_id: String,
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(
    seed: u32,
    attacker: &Value,
    target: &Value,
    defender: &Value,
    survivor: &Value,
) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "first-strike-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-first-strike-rules-v1",
        },
        "cards": {
            "north-attacker": attacker,
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-defender": defender,
            "south-site": site(),
            "south-survivor": survivor,
            "south-target": target,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-attacker"; 3],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": ["south-defender", "south-survivor", "south-target"],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit<'a>(position: &'a Value, instance_id: &str) -> Option<&'a Value> {
    position["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn in_cemetery(position: &Value, seat: &str, instance_id: &str) -> bool {
    position["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn assert_exact_replay(session: &Session) {
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

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("first-strike checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn prepare_attack(
    seed: u32,
    attacker: &Value,
    target: &Value,
    defender: &Value,
    survivor: &Value,
    join_defender: bool,
    join_survivor: bool,
) -> AttackSetup {
    let encoded = manifest(seed, attacker, target, defender, survivor);
    let mut session = Session::new(&encoded).expect("valid first-strike scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-attacker"
            && descriptor["cell"] == "C4"
    });
    let attacker_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let defender_id = join_defender.then(|| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-defender"
                && descriptor["cell"] == "C1"
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("defender identity")
            .to_owned()
    });
    let survivor_id = join_survivor.then(|| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-survivor"
                && descriptor["cell"] == "C1"
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("surviving defender identity")
            .to_owned()
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C2"
    });
    let target_id = summon["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });

    AttackSetup {
        attacker_id,
        defender_id,
        session,
        survivor_id,
        target_id,
    }
}

fn resolve_single_target(mut setup: AttackSetup) -> AttackSetup {
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    setup
}

#[test]
fn attacking_only_first_strike_should_kill_before_return_and_be_inactive_defending() {
    let vanilla = minion(json!({ "attack": 3, "defense": 3 }));
    let first_strike = minion(json!({
        "attack": 3,
        "defense": 3,
        "strikesFirstWhileAttacking": true,
    }));

    let attacking = resolve_single_target(prepare_attack(
        117,
        &first_strike,
        &vanilla,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&attacking.session);
    assert_eq!(
        unit(&position, &attacking.attacker_id).expect("surviving attacker")["damage"],
        0
    );
    assert!(in_cemetery(&position, "south", &attacking.target_id));
    let final_receipt = attacking
        .session
        .transcript()
        .last()
        .expect("fight receipt");
    let target_damage = final_receipt
        .events
        .iter()
        .position(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == attacking.target_id
        })
        .expect("first-strike damage");
    let death = final_receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("first-strike death");
    assert!(target_damage < death);
    assert!(final_receipt.events.iter().all(|event| {
        event.event_type != "damage-dealt" || event.payload["instanceId"] != attacking.attacker_id
    }));
    assert_exact_replay(&attacking.session);

    let defending = resolve_single_target(prepare_attack(
        118,
        &vanilla,
        &first_strike,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&defending.session);
    assert!(in_cemetery(&position, "north", &defending.attacker_id));
    assert!(in_cemetery(&position, "south", &defending.target_id));
    assert_exact_replay(&defending.session);
}

#[test]
fn rule_catalog_0204_defending_first_strike_should_kill_before_the_attacker_strikes() {
    let vanilla = minion(json!({ "attack": 3, "defense": 3 }));
    let defending_first_strike = minion(json!({
        "attack": 3,
        "defense": 3,
        "strikesFirstWhileDefending": true,
    }));

    let defending = resolve_single_target(prepare_attack(
        204,
        &vanilla,
        &defending_first_strike,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&defending.session);
    assert!(in_cemetery(&position, "north", &defending.attacker_id));
    assert_eq!(
        unit(&position, &defending.target_id).expect("surviving defender")["damage"],
        0
    );
    let final_receipt = defending
        .session
        .transcript()
        .last()
        .expect("fight receipt");
    let attacker_damage = final_receipt
        .events
        .iter()
        .position(|event| {
            event.event_type == "damage-dealt"
                && event.payload["instanceId"] == defending.attacker_id
        })
        .expect("defending first-strike damage");
    let death = final_receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("defending first-strike death");
    assert!(attacker_damage < death);
    assert!(final_receipt.events.iter().all(|event| {
        event.event_type != "damage-dealt" || event.payload["instanceId"] != defending.target_id
    }));
    assert_exact_replay(&defending.session);
    assert_checkpoint_round_trip(&defending.session);
}

#[test]
fn rule_catalog_0205_defending_only_first_strike_should_be_inactive_while_attacking() {
    let vanilla = minion(json!({ "attack": 3, "defense": 3 }));
    let defending_first_strike = minion(json!({
        "attack": 3,
        "defense": 3,
        "strikesFirstWhileDefending": true,
    }));

    let attacking = resolve_single_target(prepare_attack(
        205,
        &defending_first_strike,
        &vanilla,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&attacking.session);
    assert!(in_cemetery(&position, "north", &attacking.attacker_id));
    assert!(in_cemetery(&position, "south", &attacking.target_id));
    assert_exact_replay(&attacking.session);
}

#[test]
fn rule_catalog_0206_printed_first_strike_should_resolve_early_attacking_and_defending() {
    let vanilla = minion(json!({ "attack": 3, "defense": 3 }));
    let first_strike = minion(json!({
        "attack": 3,
        "defense": 3,
        "strikesFirstWhileAttacking": true,
        "strikesFirstWhileDefending": true,
    }));

    let attacking = resolve_single_target(prepare_attack(
        206,
        &first_strike,
        &vanilla,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&attacking.session);
    assert_eq!(
        unit(&position, &attacking.attacker_id).expect("surviving attacker")["damage"],
        0
    );
    assert!(in_cemetery(&position, "south", &attacking.target_id));
    assert!(
        attacking
            .session
            .transcript()
            .last()
            .expect("fight receipt")
            .events
            .iter()
            .all(|event| {
                event.event_type != "damage-dealt"
                    || event.payload["instanceId"] != attacking.attacker_id
            })
    );
    assert_exact_replay(&attacking.session);

    let defending = resolve_single_target(prepare_attack(
        207,
        &vanilla,
        &first_strike,
        &vanilla,
        &vanilla,
        false,
        false,
    ));
    let position = state(&defending.session);
    assert!(in_cemetery(&position, "north", &defending.attacker_id));
    assert_eq!(
        unit(&position, &defending.target_id).expect("surviving defender")["damage"],
        0
    );
    assert!(
        defending
            .session
            .transcript()
            .last()
            .expect("fight receipt")
            .events
            .iter()
            .all(|event| {
                event.event_type != "damage-dealt"
                    || event.payload["instanceId"] != defending.target_id
            })
    );
    assert_exact_replay(&defending.session);
    assert_checkpoint_round_trip(&defending.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct continuation proof keeps allocation, ordering, resume, and event order together"
)]
fn first_strike_deathrites_should_pause_order_and_resume_normal_return_once() {
    let deathrite = minion(json!({
        "attack": 2,
        "deathriteDamageEachUnitHere": 1,
        "defense": 3,
    }));
    let survivor = minion(json!({ "attack": 2, "defense": 10 }));
    let mut setup = prepare_attack(
        280,
        &minion(json!({
            "attack": 6,
            "defense": 10,
            "strikesFirstWhileAttacking": true,
        })),
        &deathrite,
        &deathrite,
        &survivor,
        true,
        true,
    );
    let defender_id = setup.defender_id.as_ref().expect("Deathrite defender");
    let survivor_id = setup.survivor_id.as_ref().expect("surviving defender");
    let mut deathrite_ids = [setup.target_id.clone(), defender_id.clone()];
    deathrite_ids.sort();

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    for defender_id in [defender_id, survivor_id] {
        accept_where(&mut setup.session, |descriptor| {
            descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == defender_id.as_str()
        });
    }
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    assert_eq!(state(&setup.session)["phase"], "allocate");
    while state(&setup.session)["phase"] == "allocate" {
        accept_where(&mut setup.session, |descriptor| {
            descriptor["kind"] == "allocate-strike"
                && descriptor["amount"]
                    == if deathrite_ids
                        .iter()
                        .any(|id| descriptor["targetInstanceId"] == id.as_str())
                    {
                        3
                    } else {
                        0
                    }
        });
    }

    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["kind"],
        "first-strike"
    );
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["attackerStrikesFirst"],
        true
    );
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["firstCombatantInstanceIds"],
        json!([])
    );
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["pending"]["attacker"]["instanceId"],
        setup.attacker_id
    );
    assert_eq!(
        unit(&paused, &setup.attacker_id).expect("undamaged attacker")["damage"],
        0
    );
    assert!(
        deathrite_ids
            .iter()
            .all(|id| !in_cemetery(&paused, "south", id))
    );
    assert!(
        setup
            .session
            .transcript()
            .last()
            .expect("paused allocation receipt")
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    assert_checkpoint_round_trip(&setup.session);

    let actions = setup
        .session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 2);
    let first_source = actions[0].descriptor["sourceInstanceId"]
        .as_str()
        .expect("first ordered source")
        .to_owned();
    let before_version = paused["stateVersion"].as_u64().expect("paused version");
    let (_, receipt) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == first_source
    });
    assert_eq!(receipt.state_version, before_version);
    assert_eq!(receipt.next_state_version, before_version + 1);

    let sources = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "deathrite-damage-allocated")
        .filter_map(|event| event.payload["sourceInstanceId"].as_str())
        .fold(Vec::new(), |mut unique, source| {
            if !unique.contains(&source) {
                unique.push(source);
            }
            unique
        });
    assert_eq!(sources.first().copied(), Some(first_source.as_str()));
    assert_eq!(sources.len(), 2);
    let last_deathrite = receipt
        .events
        .iter()
        .rposition(|event| event.event_type == "deathrite-damage-allocated")
        .expect("last Deathrite allocation");
    let first_death = receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("first early death");
    let return_damage = receipt
        .events
        .iter()
        .rposition(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == setup.attacker_id
        })
        .expect("normal return damage");
    assert!(last_deathrite < first_death && first_death < return_damage);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died")
            .count(),
        2
    );

    let resolved = state(&setup.session);
    assert_eq!(resolved["stateVersion"], before_version + 1);
    assert_eq!(
        unit(&resolved, &setup.attacker_id).expect("attacker")["damage"],
        4
    );
    assert_eq!(
        unit(&resolved, survivor_id).expect("surviving defender")["damage"],
        2
    );
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 20);
    assert!(
        deathrite_ids
            .iter()
            .all(|id| in_cemetery(&resolved, "south", id))
    );
    assert_eq!(resolved["phase"], "main");
    assert!(resolved["pendingCombat"].is_null());
    assert!(resolved["pendingDeathrites"].is_null());
    assert_exact_replay(&setup.session);
}

fn resolve_sleep_fight(seed: u32, attacker: &Value, sleeper: &Value) -> AttackSetup {
    let setup = prepare_attack(
        seed,
        attacker,
        sleeper,
        &minion(json!({})),
        &minion(json!({})),
        false,
        false,
    );
    assert_eq!(
        unit(&state(&setup.session), &setup.target_id).expect("Genesis sleeper")["disabledUntilDamaged"],
        true
    );
    resolve_single_target(setup)
}

#[test]
fn genesis_sleep_should_require_real_damage_and_not_strike_retroactively() {
    let attacker = minion(json!({ "attack": 2, "defense": 6 }));
    let sleeper = minion(json!({
        "attack": 5,
        "defense": 5,
        "genesisDisableSelfUntilDamaged": true,
    }));

    let ordinary = resolve_sleep_fight(119, &attacker, &sleeper);
    let position = state(&ordinary.session);
    assert_eq!(
        unit(&position, &ordinary.attacker_id).expect("attacker")["damage"],
        0
    );
    let awakened = unit(&position, &ordinary.target_id).expect("awakened sleeper");
    assert_eq!(awakened["damage"], 2);
    assert!(awakened["disabledUntilDamaged"].is_null());
    assert_eq!(
        ordinary
            .session
            .transcript()
            .last()
            .expect("ordinary fight")
            .events
            .iter()
            .filter(|event| event.event_type == "minion-awakened")
            .count(),
        1
    );
    assert_exact_replay(&ordinary.session);

    let early = resolve_sleep_fight(
        120,
        &minion(json!({
            "attack": 2,
            "defense": 6,
            "strikesFirstWhileAttacking": true,
        })),
        &sleeper,
    );
    let position = state(&early.session);
    assert_eq!(
        unit(&position, &early.attacker_id).expect("attacker")["damage"],
        5
    );
    assert_eq!(
        unit(&position, &early.target_id).expect("awakened sleeper")["damage"],
        2
    );
    assert_exact_replay(&early.session);

    let warded = resolve_sleep_fight(
        121,
        &attacker,
        &minion(json!({
            "attack": 5,
            "defense": 5,
            "genesisDisableSelfUntilDamaged": true,
            "ward": true,
        })),
    );
    let position = state(&warded.session);
    assert_eq!(
        unit(&position, &warded.target_id).expect("warded sleeper")["damage"],
        0
    );
    assert_eq!(
        unit(&position, &warded.target_id).expect("sleeping target")["disabledUntilDamaged"],
        true
    );
    assert!(
        warded
            .session
            .transcript()
            .last()
            .expect("Ward fight")
            .events
            .iter()
            .all(|event| event.event_type != "minion-awakened")
    );
    assert_exact_replay(&warded.session);
}

#[test]
fn simultaneous_strike_should_snapshot_every_target_before_awakening_an_aura() {
    let sleeper = minion(json!({
        "attack": 1,
        "defense": 10,
        "genesisDisableSelfUntilDamaged": true,
        "otherNearbyAlliesPowerBonus": 1,
    }));
    let ally = minion(json!({ "attack": 1, "defense": 2 }));
    let mut setup = prepare_attack(
        281,
        &minion(json!({
            "attack": 3,
            "defense": 10,
            "strikesFirstWhileAttacking": true,
        })),
        &sleeper,
        &minion(json!({})),
        &ally,
        false,
        true,
    );
    let ally_id = setup.survivor_id.as_ref().expect("nearby ally");
    assert!(
        setup.target_id < *ally_id,
        "fixture must awaken the aura first"
    );

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == ally_id.as_str()
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    while state(&setup.session)["phase"] == "allocate" {
        accept_where(&mut setup.session, |descriptor| {
            descriptor["kind"] == "allocate-strike"
                && descriptor["amount"]
                    == if descriptor["targetInstanceId"] == setup.target_id {
                        1
                    } else {
                        2
                    }
        });
    }

    let position = state(&setup.session);
    assert_eq!(
        unit(&position, &setup.target_id).expect("awakened aura")["damage"],
        1
    );
    assert!(in_cemetery(&position, "south", ally_id));
    assert_exact_replay(&setup.session);
}
