use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn mutate_scenario_manifest(
    manifest: &str,
    mut mutate_card: impl FnMut(&str, &mut Value),
) -> String {
    let mut value: Value = serde_json::from_str(manifest).expect("scenario manifest value");
    value
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId")
        .expect("manifest identity");
    for (card_id, card) in value["cards"].as_object_mut().expect("manifest cards") {
        mutate_card(card_id, card);
    }
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical scenario manifest")
}

fn minion_is_attack_target(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("attack actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "declare-attack"
                && action.descriptor["target"]["kind"] == "minion"
                && action.descriptor["target"]["instanceId"] == instance_id
        })
}

#[test]
fn rule_catalog_0104_stealth_hides_until_the_attacker_interacts() {
    let base = scenario_manifest(123, 3, 5, false, 1, 20);
    let manifest = mutate_scenario_manifest(&base, |card_id, card| {
        if card_id.starts_with("north-spell-") {
            card["stealth"] = json!(true);
        }
    });
    let mut setup = north_attacks_with_manifest(&manifest);

    assert!(minion_is_attack_target(
        &setup.session,
        &setup.target_instance_id
    ));
    let (_, declined) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(declined.events[0].payload["interceptWindowOpened"], false);
    assert_eq!(state(&setup.session)["phase"], "main");
    assert_eq!(
        state(&setup.session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.attacker_instance_id)
            .expect("Stealth attacker")["stealthed"],
        true
    );

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.target_instance_id
            && descriptor["path"] == json!([{ "cell": "C2", "region": "surface" }])
    });
    assert!(!minion_is_attack_target(
        &setup.session,
        &setup.attacker_instance_id
    ));
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.attacker_instance_id
            && descriptor["path"] == json!([{ "cell": "C2", "region": "surface" }])
    });
    let (_, fought) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_instance_id
    });
    let event_types: Vec<_> = fought
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert_eq!(state(&setup.session)["phase"], "main");
    assert!(!event_types.contains(&"defend-window-closed"));
    assert_eq!(
        event_types
            .iter()
            .filter(|event_type| **event_type == "stealth-lost")
            .count(),
        1
    );
    assert_eq!(
        state(&setup.session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.attacker_instance_id)
            .expect("revealed attacker")["stealthed"],
        false
    );

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.target_instance_id
            && descriptor["path"] == json!([{ "cell": "C2", "region": "surface" }])
    });
    assert!(minion_is_attack_target(
        &setup.session,
        &setup.attacker_instance_id
    ));
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_0763_stealth_site_attack_reveals_after_strike_events() {
    let base = scenario_manifest(124, 3, 5, false, 1, 20);
    let manifest = mutate_scenario_manifest(&base, |card_id, card| {
        if card_id.starts_with("north-spell-") {
            card["stealth"] = json!(true);
        }
    });
    let mut setup = north_attacks_with_manifest(&manifest);
    let (_, receipt) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    });

    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "attack-declared",
            "undefended-site-struck",
            "avatar-life-lost",
            "stealth-lost",
        ]
    );
    assert_eq!(state(&setup.session)["phase"], "main");
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_0766_disabled_raw_stealth_does_not_hide_attack_target() {
    let base = scenario_manifest(125, 3, 5, false, 1, 20);
    let manifest = mutate_scenario_manifest(&base, |card_id, card| {
        if card_id.starts_with("south-spell-") {
            card["stealth"] = json!(true);
            card["waterbound"] = json!(true);
        }
    });
    let setup = north_attacks_with_manifest(&manifest);
    let target = state(&setup.session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == setup.target_instance_id)
        .expect("disabled raw-Stealth target")
        .clone();

    assert_eq!(target["stealthed"], false);
    assert!(minion_is_attack_target(
        &setup.session,
        &setup.target_instance_id
    ));
    assert_exact_replay(&setup.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof preserves movement, source ordering, disable expiry, and replay timing"
)]
fn rule_catalog_0107_scent_hounds_permanently_remove_nearby_enemy_stealth() {
    let manifest = (1..=512)
        .map(|seed| {
            let base = scenario_manifest(seed, 2, 2, false, 1, 20);
            mutate_scenario_manifest(&base, |card_id, card| {
                if card_id.starts_with("north-spell-") {
                    card["nearbyEnemiesPermanentlyLoseStealth"] = json!(true);
                } else if let Some(ordinal) = card_id
                    .strip_prefix("south-spell-")
                    .and_then(|ordinal| ordinal.parse::<u8>().ok())
                {
                    if ordinal % 2 == 0 {
                        *card = json!({
                            "cardType": "magic",
                            "disableTargetNearbyMinionUntilNextTurn": true,
                            "manaCost": 0,
                            "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
                        });
                    } else {
                        card["gainsStealthAtEndOfTurn"] = json!(true);
                        card["spellcaster"] = json!(true);
                        card["stealth"] = json!(true);
                    }
                }
            })
        })
        .find(|manifest| {
            let preview = Session::new(manifest).expect("candidate Scent Hounds session");
            let preview_state = state(&preview);
            let cards = preview_state["cards"].as_object().expect("manifest cards");
            let hand = preview_state["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening Spellbook hand");
            hand.iter()
                .any(|card| cards[card["cardId"].as_str().expect("card id")]["cardType"] == "magic")
                && hand.iter().any(|card| {
                    cards[card["cardId"].as_str().expect("card id")]["cardType"] == "minion"
                })
        })
        .expect("seed with a South minion and Freeze in the opening hand");
    let mut session = Session::new(&manifest).expect("valid Scent Hounds scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (hound_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let hound_id = hound_summon["cardInstanceId"]
        .as_str()
        .expect("Scent Hound identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (target_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    let target_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("Stealth target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let branch = session.clone();

    let mut ordered_sources = branch.clone();
    let (second_summon, _) = accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C3"
    });
    let second_hound_id = second_summon["cardInstanceId"]
        .as_str()
        .expect("second Scent Hound identity")
        .to_owned();
    accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == hound_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (_, ordered_loss) = accept_where(&mut ordered_sources, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == target_id
            && descriptor["to"]["cell"] == "C2"
    });
    assert_eq!(
        ordered_loss
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["move-and-attack-activated", "stealth-lost"]
    );
    let lowest_hound_id = if hound_id < second_hound_id {
        &hound_id
    } else {
        &second_hound_id
    };
    assert_eq!(
        ordered_loss.events[1].payload,
        json!({
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": lowest_hound_id,
        })
    );
    assert_exact_replay(&ordered_sources);

    session = branch;
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == hound_id
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
    let (_, movement_loss) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == target_id
            && descriptor["to"]["cell"] == "C2"
    });
    assert_eq!(
        movement_loss
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["move-and-attack-activated", "stealth-lost"]
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["target"]["instanceId"] == hound_id
    });
    let (_, disabled_end) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        disabled_end
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["stealth-gained", "turn-ended", "turn-started"]
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, expiry) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        expiry
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "turn-ended",
            "minion-disable-expired",
            "stealth-lost",
            "turn-started",
        ]
    );
    assert_eq!(
        expiry.events[2].payload,
        json!({
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": hound_id,
        })
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_id)
            .expect("Stealth target")["stealthed"],
        false
    );
    assert_exact_replay(&session);
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
fn rule_catalog_0772_zero_power_avatars_emit_no_zero_damage_or_life_loss() {
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
fn rule_catalog_0862_lethal_requires_positive_minion_damage() {
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
fn rule_catalog_0861_deaths_door_prevents_same_turn_damage_and_later_draw() {
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
fn rule_catalog_0773_terminal_outcome_exposes_winner_and_loser() {
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
fn rule_catalog_0786_surviving_minion_damage_persists_until_end_phase() {
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
fn rule_catalog_0793_later_undefended_site_strikes_skip_deaths_door_death_blows() {
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

fn must_attack_unit_manifest(seed: u32) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "must-attack-unit-not-site" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-must-attack-unit-not-site-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-site": site,
            "north-source": {
                "attack": 2,
                "cardType": "minion",
                "charge": true,
                "defense": 2,
                "manaCost": 0,
                "mustAttackAUnitIfAble": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": avatar,
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site,
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
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn after_must_attack_unit_and_site_setup(seed: u32) -> Session {
    let mut session = Session::new(&must_attack_unit_manifest(seed)).expect("valid session");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
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
    reason = "must-attack unit-vs-site scenario proof keeps setup and assertions inline"
)]
fn rule_catalog_0917_must_attack_a_unit_if_able_excludes_site_targets_when_both_are_in_range() {
    let mut session = after_must_attack_unit_and_site_setup(917);
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
    let site_instance_id = state(&session)["realm"]["sites"]["C3"]["instanceId"].clone();
    let legal = session.legal_actions().expect("mandatory unit attacks");
    assert!(!legal.is_empty());
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == source_id
            && action.descriptor["to"]["cell"] == "C3"
    }));
    let checkpoint = session.clone();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C3"
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
    assert!(
        attack_actions.iter().all(|action| {
            action.descriptor["kind"] != "declare-attack"
                || action.descriptor["target"]["kind"] == "minion"
        }),
        "must attack a unit if able must not offer a site Declare Attack"
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
    assert_ne!(
        attack_actions[0].descriptor["target"]["instanceId"],
        site_instance_id
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

fn enemies_must_attack_this_manifest(seed: u32) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "enemies-must-attack-this-not-site" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-enemies-must-attack-this-not-site-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-site": site,
            "north-source": {
                "attack": 2,
                "cardType": "minion",
                "charge": true,
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": avatar,
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "enemiesMustAttackThisIfAble": true,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site,
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
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn after_enemies_must_attack_this_and_site_setup(seed: u32) -> Session {
    let mut session =
        Session::new(&enemies_must_attack_this_manifest(seed)).expect("valid session");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
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
    reason = "enemies-must-attack-this unit-vs-site scenario proof keeps setup and assertions inline"
)]
fn rule_catalog_0927_enemies_must_attack_this_if_able_excludes_site_targets_when_both_are_in_range()
{
    let mut session = after_enemies_must_attack_this_and_site_setup(927);
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
    let site_instance_id = state(&session)["realm"]["sites"]["C3"]["instanceId"].clone();
    let legal = session.legal_actions().expect("mandatory forced attacks");
    assert!(!legal.is_empty());
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == source_id
            && action.descriptor["to"]["cell"] == "C3"
    }));
    let checkpoint = session.clone();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C3"
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
    assert!(
        attack_actions.iter().all(|action| {
            action.descriptor["kind"] != "declare-attack"
                || action.descriptor["target"]["kind"] == "minion"
        }),
        "enemies must attack this if able must not offer a site Declare Attack"
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
    assert_ne!(
        attack_actions[0].descriptor["target"]["instanceId"],
        site_instance_id
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

fn nearby_must_attack_mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyMinionsMustAttackIfAble": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn nearby_must_attack_mask_manifest(seed: u32) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "nearby-must-attack-site-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-nearby-must-attack-site-target-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-site": site,
            "north-source": {
                "attack": 2,
                "cardType": "minion",
                "charge": true,
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": avatar,
            "south-mask": nearby_must_attack_mask(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site,
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
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn nearby_must_attack_mask_opening_manifest() -> String {
    (1..=4096)
        .map(nearby_must_attack_mask_manifest)
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

fn after_nearby_must_attack_mask_and_site_setup() -> Session {
    let mut session =
        Session::new(&nearby_must_attack_mask_opening_manifest()).expect("valid session");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
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
fn rule_catalog_0937_nearby_must_attack_if_able_still_offers_site_targets_when_a_unit_is_also_in_range()
 {
    let mut session = after_nearby_must_attack_mask_and_site_setup();
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
    let site_instance_id = state(&session)["realm"]["sites"]["C3"]["instanceId"].clone();
    let legal = session.legal_actions().expect("mandatory nearby attacks");
    assert!(!legal.is_empty());
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == source_id
    }));
    assert!(legal.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack" && action.descriptor["to"]["cell"] == "C3"
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id
            && descriptor["to"]["cell"] == "C3"
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
    assert!(
        attack_actions.iter().any(|action| {
            action.descriptor["kind"] == "declare-attack"
                && action.descriptor["target"]["kind"] == "site"
                && action.descriptor["target"]["instanceId"] == site_instance_id
        }),
        "nearby-must-attack must still offer a site Declare Attack when a unit is also in range"
    );
    assert!(attack_actions.iter().any(|action| {
        action.descriptor["kind"] == "declare-attack"
            && action.descriptor["target"]["kind"] == "minion"
            && action.descriptor["target"]["instanceId"] == target_id
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_instance_id
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert_eq!(
        receipt
            .events
            .iter()
            .find(|event| event.event_type == "undefended-site-struck")
            .expect("site strike")
            .payload["amount"],
        2
    );
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    assert_eq!(state(&session)["phase"], "main");
    assert_exact_replay(&session);
}

fn avatar_card() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site_card(extra: Value) -> Value {
    let mut value = json!({ "cardType": "site", "elements": ["earth"] });
    let Value::Object(extra) = extra else {
        panic!("extra site facts must be an object");
    };
    value.as_object_mut().expect("site facts").extend(extra);
    value
}

fn minion_card(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn combat_deathrite_manifest(seed: u32) -> String {
    let cards = json!({
        "north-attacker": minion_card(json!({
            "attack": 4,
            "lethal": true,
            "movementBonus": 1,
        })),
        "north-avatar": avatar_card(),
        "north-shooter": minion_card(json!({ "ranged": true })),
        "north-site": site_card(json!({ "rangedUnitsHereRangeBonus": 1 })),
        "south-aura": minion_card(json!({
            "defense": 1,
            "otherNearbyAlliesPowerBonus": 1,
            "summonToAnySite": true,
        })),
        "south-avatar": avatar_card(),
        "south-deathrite-a": minion_card(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        "south-deathrite-b": minion_card(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        "south-site": site_card(json!({})),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "combat-move-attack-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-combat-move-attack-deathrite-withheld-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": ["north-shooter", "north-shooter", "north-attacker"],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": ["south-aura", "south-deathrite-a", "south-deathrite-b"],
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

fn realm_unit<'a>(position: &'a Value, instance_id: &str) -> Option<&'a Value> {
    position["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("combat checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
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

fn combat_unit<'a>(position: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(position, instance_id).expect("expected unit")
}

fn fire_south_projectile(session: &mut Session, shooter_id: &str, target_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["direction"] == "south"
            && descriptor["shooterInstanceId"] == shooter_id
            && descriptor["hit"]["instanceId"] == target_id
    })
    .1
}

fn move_and_attack_unit_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "move-and-attack")
        .map(|action| {
            action.descriptor["unitInstanceId"]
                .as_str()
                .expect("move-and-attack unit")
                .to_owned()
        })
        .collect()
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves move-and-attack stays withheld until Deathrites are ordered"
)]
fn rule_catalog_1019_move_and_attack_withheld_during_pending_deathrite_order() {
    let encoded = combat_deathrite_manifest(198);
    let mut session = Session::new(&encoded).expect("valid combat Deathrite withheld scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let mut shooters = Vec::new();
    for _ in 0..2 {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-shooter"
                && descriptor["cell"] == "C4"
        });
        shooters.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("shooter identity")
                .to_owned(),
        );
    }
    let (attacker, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-attacker"
            && descriptor["cell"] == "C4"
    });
    let attacker_id = attacker["cardInstanceId"]
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let mut south_ids = Vec::new();
    for card_id in ["south-aura", "south-deathrite-a", "south-deathrite-b"] {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C2"
        });
        south_ids.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("South minion identity")
                .to_owned(),
        );
    }
    let aura_id = south_ids[0].clone();
    let mut deathrite_ids = [south_ids[1].clone(), south_ids[2].clone()];
    deathrite_ids.sort_unstable();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    fire_south_projectile(&mut session, &shooters[0], &south_ids[1]);
    fire_south_projectile(&mut session, &shooters[1], &south_ids[2]);
    assert_eq!(combat_unit(&state(&session), &south_ids[1])["damage"], 1);
    assert_eq!(combat_unit(&state(&session), &south_ids[2])["damage"], 1);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    while state(&session)["phase"] == "movement" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == aura_id
    });
    while state(&session)["phase"] == "defend" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "close-defend"
        });
    }
    while state(&session)["phase"] == "allocate" {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "allocate-strike"
                && descriptor["amount"]
                    .as_u64()
                    .is_some_and(|amount| amount > 0)
                && descriptor["targetInstanceId"] == aura_id
        });
    }

    let paused = state(&session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(realm_unit(&paused, &aura_id).is_none());
    assert!(
        deathrite_ids
            .iter()
            .all(|instance_id| realm_unit(&paused, instance_id).is_none())
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "move-and-attack"),
        "Move and Attack must stay withheld until the combat Deathrite chain completes"
    );
    assert!(
        session
            .legal_actions()
            .expect("Deathrite order actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "order-deathrites")
    );
    assert_checkpoint_round_trip(&session);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    assert_eq!(
        event_types(&resolved),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "minion-died",
        ]
    );

    while matches!(state(&session)["phase"].as_str(), Some("attack" | "defend" | "allocate" | "intercept")) {
        if try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "close-defend"
        })
        .is_some()
        {
            continue;
        }
        if try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "allocate-strike"
                && descriptor["amount"]
                    .as_u64()
                    .is_some_and(|amount| amount > 0)
        })
        .is_some()
        {
            continue;
        }
        if try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        })
        .is_some()
        {
            continue;
        }
        break;
    }

    let resumed = state(&session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        !move_and_attack_unit_ids(&session).is_empty(),
        "Move and Attack must be offered again once deathrite-order clears"
    );
    assert_exact_replay(&session);
}
