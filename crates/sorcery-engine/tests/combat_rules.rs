use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameOutcome};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

struct AttackSetup {
    attacker_instance_id: String,
    session: Session,
    target_instance_id: String,
}

struct AvatarAttackSetup {
    north_avatar_instance_id: String,
    north_minion_instance_id: String,
    session: Session,
    south_avatar_instance_id: String,
}

fn scenario_manifest(
    seed: u32,
    minion_attack: u64,
    minion_defense: u64,
    minion_lethal: bool,
    avatar_attack: u64,
    avatar_life: u64,
) -> String {
    let manifest_json = synthetic_demo_manifest_json(seed).expect("synthetic manifest");
    let mut manifest: Value = serde_json::from_str(&manifest_json).expect("manifest value");
    let body = manifest.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    for card in body["cards"]
        .as_object_mut()
        .expect("manifest cards")
        .values_mut()
    {
        match card["cardType"].as_str() {
            Some("avatar") => {
                card["attack"] = json!(avatar_attack);
                card["life"] = json!(avatar_life);
            }
            Some("minion") => {
                card["attack"] = json!(minion_attack);
                card["defense"] = json!(minion_defense);
                if minion_lethal {
                    card["lethal"] = json!(true);
                } else if let Some(card) = card.as_object_mut() {
                    card.remove("lethal");
                }
            }
            _ => {}
        }
    }
    let manifest_id = identity_hash(&manifest).expect("scenario manifest identity");
    manifest["manifestId"] = json!(manifest_id);
    canonical_json(&manifest).expect("canonical scenario manifest")
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

fn north_attacks_at_c2(seed: u32, minion_defense: u64, avatar_life: u64) -> AttackSetup {
    let manifest = scenario_manifest(seed, 1, minion_defense, false, 1, avatar_life);
    north_attacks_with_manifest(&manifest)
}

fn north_attacks_with_manifest(manifest: &str) -> AttackSetup {
    let mut session = Session::new(manifest).expect("valid scenario session");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let attacker_instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C2"
    });
    let target_instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("target instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });

    AttackSetup {
        attacker_instance_id,
        session,
        target_instance_id,
    }
}

fn north_avatar_attacks_south_at_c2(seed: u32, avatar_attack: u64) -> AvatarAttackSetup {
    let manifest = scenario_manifest(seed, 1, 1, false, avatar_attack, 1);
    let mut session = Session::new(&manifest).expect("valid Avatar combat session");
    keep(&mut session);
    keep(&mut session);
    let opening = state(&session);
    let north_avatar_instance_id = opening["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north Avatar instance identity")
        .to_owned();
    let south_avatar_instance_id = opening["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("south Avatar instance identity")
        .to_owned();

    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let north_minion_instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("north minion instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_minion_instance_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_minion_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_avatar_instance_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == south_avatar_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_avatar_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    AvatarAttackSetup {
        north_avatar_instance_id,
        north_minion_instance_id,
        session,
        south_avatar_instance_id,
    }
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
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

fn replay_game(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("valid replay game");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .is_ok_and(|action| action.action_id == receipt.action_id)
            })
            .expect("recorded engine-issued action");
        game.apply_action(&action).expect("replay action");
    }
    game
}

#[test]
fn zero_power_avatars_should_not_emit_zero_damage_or_life_loss() {
    let AvatarAttackSetup {
        mut session,
        south_avatar_instance_id,
        ..
    } = north_avatar_attacks_south_at_c2(68, 0);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["instanceId"] == south_avatar_instance_id
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });

    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "defend-window-closed",
            "fight-started",
            "strike-damage-allocated",
        ]
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct rule proof keeps each exact combat transcript together"
)]
fn lethal_requires_positive_minion_damage() {
    let resolve = |attack, seed| {
        let manifest = scenario_manifest(seed, attack, 5, true, 1, 20);
        let mut setup = north_attacks_with_manifest(&manifest);
        accept_where(&mut setup.session, |descriptor| {
            descriptor["kind"] == "declare-attack"
                && descriptor["target"]["kind"] == "minion"
                && descriptor["target"]["instanceId"] == setup.target_instance_id
        });
        let (_, fight) = accept_where(&mut setup.session, |descriptor| {
            descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
        });
        (setup, fight)
    };

    let (positive, fight) = resolve(1, 45);
    let positive_state = state(&positive.session);
    let north_card_id = positive_state["players"]["north"]["cemetery"][0]["cardId"].clone();
    let south_card_id = positive_state["players"]["south"]["cemetery"][0]["cardId"].clone();
    assert_eq!(
        event_values(&fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": positive.attacker_instance_id,
                    "combatantInstanceIds": [positive.target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "strikerInstanceId": positive.attacker_instance_id,
                    "targetInstanceId": positive.target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": positive.attacker_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": positive.target_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "cardId": north_card_id,
                    "instanceId": positive.attacker_instance_id,
                    "owner": "north",
                },
                "type": "minion-died",
            }),
            json!({
                "payload": {
                    "cardId": south_card_id,
                    "instanceId": positive.target_instance_id,
                    "owner": "south",
                },
                "type": "minion-died",
            }),
        ]
    );
    assert_eq!(
        positive_state["players"]["north"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        positive_state["players"]["south"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_exact_replay(&positive.session);

    let (zero, fight) = resolve(0, 44);
    assert_eq!(
        event_values(&fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": zero.attacker_instance_id,
                    "combatantInstanceIds": [zero.target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 0,
                    "strikerInstanceId": zero.attacker_instance_id,
                    "targetInstanceId": zero.target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 0,
                    "amount": 0,
                    "direct": true,
                    "instanceId": zero.attacker_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "accumulated": 0,
                    "amount": 0,
                    "direct": true,
                    "instanceId": zero.target_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
        ]
    );
    let zero_state = state(&zero.session);
    assert_eq!(
        zero_state["realm"]["units"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(
        zero_state["players"]["north"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        zero_state["players"]["south"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_exact_replay(&zero.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct rule proof keeps the exact three-fight transcript together"
)]
fn deaths_door_prevents_same_turn_damage_and_later_simultaneous_death_blows_draw() {
    let AvatarAttackSetup {
        north_avatar_instance_id,
        north_minion_instance_id,
        mut session,
        south_avatar_instance_id,
    } = north_avatar_attacks_south_at_c2(67, 2);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["instanceId"] == south_avatar_instance_id
    });
    let (_, first_fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    assert_eq!(
        event_values(&first_fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": north_avatar_instance_id,
                    "combatantInstanceIds": [south_avatar_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "strikerInstanceId": north_avatar_instance_id,
                    "targetInstanceId": south_avatar_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "direct": true,
                    "instanceId": north_avatar_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": { "amount": 1, "life": 0, "seat": "north" },
                "type": "avatar-life-lost",
            }),
            json!({
                "payload": { "seat": "north", "turnNumber": 7 },
                "type": "avatar-reached-deaths-door",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "direct": true,
                    "instanceId": south_avatar_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": { "amount": 1, "life": 0, "seat": "south" },
                "type": "avatar-life-lost",
            }),
            json!({
                "payload": { "seat": "south", "turnNumber": 7 },
                "type": "avatar-reached-deaths-door",
            }),
        ]
    );
    let first_state = state(&session);
    assert_eq!(first_state["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(first_state["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(
        first_state["players"]["north"]["avatar"]["deathDoorTurn"],
        7
    );
    assert_eq!(
        first_state["players"]["south"]["avatar"]["deathDoorTurn"],
        7
    );
    assert_eq!(first_state["terminal"], json!({ "status": "active" }));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_minion_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["instanceId"] == south_avatar_instance_id
    });
    let (_, prevented) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let dead_minion_card_id = state(&session)["players"]["north"]["cemetery"][0]["cardId"].clone();
    assert_eq!(
        event_values(&prevented),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": north_minion_instance_id,
                    "combatantInstanceIds": [south_avatar_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "strikerInstanceId": north_minion_instance_id,
                    "targetInstanceId": south_avatar_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 2,
                    "amount": 2,
                    "direct": true,
                    "instanceId": north_minion_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "amount": 0,
                    "attemptedAmount": 1,
                    "direct": true,
                    "instanceId": south_avatar_instance_id,
                    "prevented": true,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "cardId": dead_minion_card_id,
                    "instanceId": north_minion_instance_id,
                    "owner": "north",
                },
                "type": "minion-died",
            }),
        ]
    );
    assert_eq!(state(&session)["terminal"], json!({ "status": "active" }));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == south_avatar_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["instanceId"] == north_avatar_instance_id
    });
    let (_, death_blows) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    assert_eq!(
        event_values(&death_blows),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": south_avatar_instance_id,
                    "combatantInstanceIds": [north_avatar_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "strikerInstanceId": south_avatar_instance_id,
                    "targetInstanceId": north_avatar_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "direct": true,
                    "instanceId": south_avatar_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": { "instanceId": south_avatar_instance_id, "seat": "south" },
                "type": "death-blow",
            }),
            json!({
                "payload": {
                    "amount": 2,
                    "direct": true,
                    "instanceId": north_avatar_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": { "instanceId": north_avatar_instance_id, "seat": "north" },
                "type": "death-blow",
            }),
            json!({
                "payload": { "reason": "simultaneous_avatar_defeat", "result": "draw" },
                "type": "game-ended",
            }),
        ]
    );
    assert_eq!(
        state(&session)["terminal"],
        json!({
            "reason": "simultaneous_avatar_defeat",
            "result": "draw",
            "status": "finished",
        })
    );
    assert_eq!(replay_game(&session).outcome(), Some(GameOutcome::Draw));
    assert_exact_replay(&session);
}

#[test]
fn terminal_outcome_exposes_winner_and_loser() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid checked-in TypeScript parity fixture");
    let game = fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .expect("seed-31 fixture game");
    let manifest_json = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let action_ids: Vec<IdentityHash> =
        serde_json::from_value(game["actionIds"].clone()).expect("fixture action identities");
    let session = Session::replay(&manifest_json, &action_ids).expect("fixture replay");
    assert_eq!(
        replay_game(&session).outcome(),
        Some(GameOutcome::Win {
            loser: Seat::North,
            winner: Seat::South,
        })
    );
}

#[test]
fn surviving_minion_damage_should_persist_until_end_phase() {
    let AttackSetup {
        attacker_instance_id,
        mut session,
        target_instance_id,
    } = north_attacks_at_c2(61, 2, 20);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });

    assert_eq!(
        event_values(&fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": attacker_instance_id,
                    "combatantInstanceIds": [target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "strikerInstanceId": attacker_instance_id,
                    "targetInstanceId": target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": attacker_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": target_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
        ]
    );
    let damaged = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["location"] == "C2")
        .map(|unit| unit["damage"].as_u64().expect("unit damage"))
        .collect::<Vec<_>>();
    assert_eq!(damaged, [1, 1]);

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_values(&ended),
        vec![
            json!({
                "payload": { "seat": "north", "turnNumber": 5 },
                "type": "turn-ended",
            }),
            json!({
                "payload": { "drawSkipped": false, "seat": "south", "turnNumber": 6 },
                "type": "turn-started",
            }),
        ]
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["damage"] == 0)
    );
    assert_exact_replay(&session);
}

#[test]
fn later_undefended_site_strikes_should_not_deliver_deaths_door_death_blows() {
    let AttackSetup {
        attacker_instance_id,
        mut session,
        ..
    } = north_attacks_at_c2(71, 1, 1);
    let site_instance_id = state(&session)["realm"]["sites"]["C2"]["instanceId"]
        .as_str()
        .expect("C2 site instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_instance_id
    });
    let (_, first_strike) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert_eq!(
        event_values(&first_strike),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": false },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "attackerInstanceId": attacker_instance_id,
                    "cell": "C2",
                    "siteInstanceId": site_instance_id,
                },
                "type": "undefended-site-struck",
            }),
            json!({
                "payload": { "amount": 1, "life": 0, "seat": "south" },
                "type": "avatar-life-lost",
            }),
            json!({
                "payload": { "seat": "south", "turnNumber": 5 },
                "type": "avatar-reached-deaths-door",
            }),
        ]
    );
    let first_state = state(&session);
    assert_eq!(first_state["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(
        first_state["players"]["south"]["avatar"]["deathDoorTurn"],
        5
    );
    assert_eq!(first_state["terminal"], json!({ "status": "active" }));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_instance_id
    });
    let (_, later_strike) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });

    assert_eq!(
        event_values(&later_strike),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": false },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "attackerInstanceId": attacker_instance_id,
                    "cell": "C2",
                    "siteInstanceId": site_instance_id,
                },
                "type": "undefended-site-struck",
            }),
        ]
    );
    let later_state = state(&session);
    assert_eq!(later_state["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(later_state["terminal"], json!({ "status": "active" }));
    assert_exact_replay(&session);
}
