use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::Game;
use sorcery_engine::session::{Session, StepResult};

fn avatar(draw_spell: bool, life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": draw_spell,
        "life": life,
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

fn manifest_value(
    seed: u32,
    north_avatar: &Value,
    north_minion: &Value,
    south_minion: &Value,
    north_atlas_count: usize,
    north_spellbook_count: usize,
    south_spellbook_count: usize,
) -> Value {
    json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-draw-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-draw-rules-v1",
        },
        "cards": {
            "north-avatar": north_avatar,
            "north-minion": north_minion,
            "north-site": site(),
            "south-avatar": avatar(false, 20),
            "south-minion": south_minion,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas_count],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; north_spellbook_count],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; south_spellbook_count],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn scenario_manifest(
    seed: u32,
    north_avatar: &Value,
    north_minion: &Value,
    south_minion: &Value,
    north_atlas_count: usize,
    north_spellbook_count: usize,
    south_spellbook_count: usize,
) -> String {
    finish_manifest(manifest_value(
        seed,
        north_avatar,
        north_minion,
        south_minion,
        north_atlas_count,
        north_spellbook_count,
        south_spellbook_count,
    ))
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

fn first_main(manifest: &str) -> Session {
    let mut session = opening_checkpoint(manifest);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn opening_checkpoint(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    session
}

fn private_site_genesis_manifest(seed: u32, facts: &Value, spellbook_count: usize) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        4,
        spellbook_count,
        4,
    );
    value["cards"]["north-site"]
        .as_object_mut()
        .expect("north site facts")
        .extend(facts.as_object().expect("Genesis facts").clone());
    finish_manifest(value)
}

fn play_private_genesis_site(manifest: &str) -> (Session, Value, Receipt) {
    let mut session = opening_checkpoint(manifest);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    (session, descriptor, receipt)
}

fn north_second_main(mut session: Session) -> Session {
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
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    assert!(session.verify_replay().expect("verified exact replay"));
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("captured checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
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
fn rule_catalog_0792_avatar_draw_spell_pays_tap_keeps_identity_private() {
    let manifest = scenario_manifest(30, &avatar(true, 20), &minion(1, 1), &minion(1, 1), 6, 5, 5);
    let mut session = north_second_main(first_main(&manifest));
    let before = state(&session);
    let opponent_before = replay_game(&session).observe(Seat::South);
    let drawn_id = before["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw-spell"
    });
    let after = state(&session);

    assert_eq!(after["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(after["players"]["north"]["avatar"]["lastInteractedTurn"], 3);
    assert_eq!(event_types(&receipt), ["spell-drawn"]);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn_id)
    );
    assert_eq!(replay_game(&session).observe(Seat::South), opponent_before);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0796_genesis_draw_site_private_identity_deck_out_after_summon() {
    let mut genesis = minion(0, 0);
    genesis["genesisDrawSite"] = json!(true);
    let manifest = scenario_manifest(40, &avatar(false, 20), &genesis, &minion(1, 1), 4, 4, 4);
    let mut session = first_main(&manifest);
    let before = state(&session);
    let drawn_id = before["players"]["north"]["atlas"][0]["instanceId"]
        .as_str()
        .expect("top Atlas identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });

    assert_eq!(event_types(&receipt), ["minion-summoned", "site-drawn"]);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn_id)
    );
    assert_exact_replay(&session);

    let empty = scenario_manifest(41, &avatar(false, 20), &genesis, &minion(1, 1), 3, 4, 4);
    let mut session = first_main(&empty);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "game-ended"]);
    assert_eq!(
        state(&session)["realm"]["units"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(state(&session)["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0838_genesis_draw_spell_private_identity_deck_out_after_summon() {
    let mut genesis = minion(0, 0);
    genesis["genesisDrawSpells"] = json!(1);
    let manifest = scenario_manifest(127, &avatar(false, 20), &genesis, &minion(1, 1), 4, 4, 4);
    let mut session = first_main(&manifest);
    let before = state(&session);
    let drawn_id = before["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });

    assert_eq!(event_types(&receipt), ["minion-summoned", "spell-drawn"]);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn_id)
    );
    assert_exact_replay(&session);

    let empty = scenario_manifest(128, &avatar(false, 20), &genesis, &minion(1, 1), 4, 3, 4);
    let mut session = first_main(&empty);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "game-ended"]);
    assert_eq!(state(&session)["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0839_numeric_genesis_spell_draws_preserve_top_order() {
    let mut genesis = minion(0, 0);
    genesis["genesisDrawSpells"] = json!(3);
    let manifest = scenario_manifest(129, &avatar(false, 20), &genesis, &minion(1, 1), 4, 6, 4);
    let mut session = first_main(&manifest);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(3)
        .map(|card| card["instanceId"].clone())
        .collect();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("Spellbook hand");

    assert_eq!(
        event_types(&receipt),
        [
            "minion-summoned",
            "spell-drawn",
            "spell-drawn",
            "spell-drawn"
        ]
    );
    let actual: Vec<_> = hand[hand.len() - 3..]
        .iter()
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(actual, expected);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0840_numeric_genesis_spell_draws_exhaust_before_deck_out() {
    let mut genesis = minion(0, 0);
    genesis["genesisDrawSpells"] = json!(3);
    for remaining in 0..=2 {
        let manifest = scenario_manifest(
            140 + remaining,
            &avatar(false, 20),
            &genesis,
            &minion(1, 1),
            4,
            3 + usize::try_from(remaining).expect("small remaining count"),
            4,
        );
        let mut session = first_main(&manifest);
        let before = state(&session);
        let expected = before["players"]["north"]["spellbook"]
            .as_array()
            .expect("Spellbook")
            .clone();
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
        });
        let after = state(&session);
        let event_types = event_types(&receipt);

        assert_eq!(event_types.first(), Some(&"minion-summoned"));
        assert_eq!(event_types.last(), Some(&"game-ended"));
        assert_eq!(
            event_types
                .iter()
                .filter(|kind| **kind == "spell-drawn")
                .count(),
            expected.len()
        );
        assert_eq!(after["players"]["north"]["spellbook"], json!([]));
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .expect("Spellbook hand")
                    .contains(&card)
            );
        }
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_exact_replay(&session);
    }
}

fn mixed_genesis_manifest(seed: u32, life: u8) -> String {
    let mut loss = minion(1, 1);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(seed, &avatar(false, life), &loss, &minion(1, 1), 4, 3, 4);
    let mut heal = minion(1, 1);
    heal["genesisHealController"] = json!(2);
    value["cards"]
        .as_object_mut()
        .expect("cards object")
        .insert("north-healer".to_owned(), heal);
    value["decks"]["north"]["spellbook"] = json!(["north-minion", "north-healer", "north-minion"]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0841_genesis_life_loss_reaches_but_not_crosses_deaths_door() {
    let mut loss = minion(1, 1);
    loss["genesisLoseControllerLife"] = json!(2);
    let manifest = scenario_manifest(226, &avatar(false, 2), &loss, &minion(1, 1), 4, 4, 4);
    let mut session = first_main(&manifest);
    let (_, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let deaths_door_turn = state(&session)["players"]["north"]["avatar"]["deathDoorTurn"].clone();

    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(
        event_types(&first),
        [
            "minion-summoned",
            "avatar-life-lost",
            "avatar-reached-deaths-door"
        ]
    );
    assert_eq!(state(&session)["terminal"]["status"], "active");

    let (_, second) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["deathDoorTurn"],
        deaths_door_turn
    );
    assert_eq!(event_types(&second), ["minion-summoned"]);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0842_genesis_heal_caps_at_max_and_skips_deaths_door() {
    let manifest = mixed_genesis_manifest(227, 3);
    let mut session = first_main(&manifest);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-minion"
    });
    let (_, healed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-healer"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 3);
    assert_eq!(event_types(&healed), ["minion-summoned", "avatar-healed"]);
    assert_eq!(healed.events[1].payload["amount"], 2);
    assert_exact_replay(&session);

    let manifest = mixed_genesis_manifest(228, 2);
    let mut session = first_main(&manifest);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-minion"
    });
    let (_, not_healed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-healer"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(event_types(&not_healed), ["minion-summoned"]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0843_undamaged_zero_defense_genesis_survives_until_positive_damage() {
    let mut attacker = minion(1, 2);
    attacker["charge"] = json!(true);
    attacker["summonToAnySite"] = json!(true);
    let mut target = minion(0, 0);
    target["genesisDrawSpells"] = json!(1);
    let manifest = scenario_manifest(131, &avatar(false, 20), &attacker, &target, 5, 5, 4);
    let mut session = first_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned_target, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "south-minion"
    });
    let target_id = summoned_target["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id && unit["damage"] == 0)
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
    let attacker_id = summoned_attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });

    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert!(fight.events.iter().any(|event| {
        event.event_type == "damage-dealt"
            && event.payload["instanceId"] == target_id
            && event.payload["amount"] == 1
    }));
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers issued branches, Ward, checkpoint, and replay parity"
)]
fn rule_catalog_0856_optional_targeted_genesis_damage_issues_decline_and_nearby_branches() {
    let mut vile_imp = minion(2, 2);
    vile_imp["genesisMayDamageTargetAdjacentUnit"] = json!(2);
    let mut warded_enemy = minion(1, 2);
    warded_enemy["summonToAnySite"] = json!(true);
    warded_enemy["ward"] = json!(true);
    let manifest = scenario_manifest(391, &avatar(false, 20), &vile_imp, &warded_enemy, 5, 5, 5);
    let mut checkpoint = first_main(&manifest);
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    let before = state(&checkpoint);
    let source_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("Vile Imp identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    let choices: Vec<_> = checkpoint
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardInstanceId"] == source_id
                && action.descriptor["cell"] == "C4"
        })
        .collect();
    assert_eq!(choices.len(), 4);
    assert_eq!(choices[0].descriptor["genesisDamageChoice"], "decline");
    assert!(choices[0].descriptor.get("genesisDamageTarget").is_none());
    assert_eq!(
        choices[0].label,
        "Summon north-minion at C4 (0 mana); decline Genesis"
    );
    let mut expected_target_ids = [source_id.clone(), avatar_id.clone(), enemy_id.clone()];
    expected_target_ids.sort();
    assert_eq!(
        choices[1..]
            .iter()
            .map(|action| {
                assert_eq!(action.descriptor["genesisDamageChoice"], "target");
                action.descriptor["genesisDamageTarget"]["instanceId"]
                    .as_str()
                    .expect("target identity")
            })
            .collect::<Vec<_>>(),
        expected_target_ids
    );
    assert_checkpoint_round_trip(&checkpoint);

    let mut declined = checkpoint.clone();
    let (_, declined_receipt) = accept_where(&mut declined, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisDamageChoice"] == "decline"
    });
    assert_eq!(event_types(&declined_receipt), ["minion-summoned"]);
    assert_exact_replay(&declined);

    let mut avatar_targeted = checkpoint.clone();
    let (_, avatar_receipt) = accept_where(&mut avatar_targeted, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisDamageTarget"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&avatar_receipt),
        [
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost"
        ]
    );
    assert_eq!(
        avatar_receipt.events[1].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": source_id,
            "targetInstanceId": avatar_id,
        })
    );
    assert_eq!(
        state(&avatar_targeted)["players"]["north"]["avatar"]["life"],
        18
    );
    assert_exact_replay(&avatar_targeted);

    let mut warded = checkpoint;
    let (_, warded_receipt) = accept_where(&mut warded, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisDamageTarget"]["instanceId"] == enemy_id
    });
    assert_eq!(
        event_types(&warded_receipt),
        [
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "ward-broken"
        ]
    );
    let warded_state = state(&warded);
    let survivor = warded_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == enemy_id)
        .expect("warded enemy survives");
    assert_eq!(survivor["damage"], 0);
    assert_eq!(survivor["warded"], false);
    assert_exact_replay(&warded);
}

fn targeted_genesis_alt_payment_checkpoint() -> (Session, String, String) {
    let mut damager = minion(2, 2);
    damager["genesisMayDamageTargetAdjacentUnit"] = json!(2);
    damager["discardRandomCardInsteadOfMana"] = json!(true);
    damager["manaCost"] = json!(3);
    let mut warded_enemy = minion(1, 2);
    warded_enemy["summonToAnySite"] = json!(true);
    warded_enemy["ward"] = json!(true);
    let manifest = scenario_manifest(481, &avatar(false, 20), &damager, &warded_enemy, 5, 5, 5);
    let mut checkpoint = first_main(&manifest);
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before = state(&checkpoint);
    let source_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("damager identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    (checkpoint, source_id, avatar_id)
}

#[test]
fn rule_catalog_0481_targeted_genesis_alt_payment_decline_discards_and_summons() {
    let (checkpoint, source_id, _) = targeted_genesis_alt_payment_checkpoint();
    let alt_summons: Vec<_> = checkpoint
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardInstanceId"] == source_id
                && action.descriptor["cell"] == "C4"
                && action.descriptor["paymentMode"] == "random-card-discard"
        })
        .collect();
    assert!(
        alt_summons
            .iter()
            .any(|action| action.descriptor["genesisDamageChoice"] == "decline")
    );
    let mut session = checkpoint;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["paymentMode"] == "random-card-discard"
            && descriptor["genesisDamageChoice"] == "decline"
    });
    assert_eq!(event_types(&receipt), ["card-discarded", "minion-summoned"]);
    assert_eq!(receipt.events[1].payload["manaPaid"], 0);
    assert_eq!(receipt.random_draws.len(), 1);
    assert_eq!(
        receipt.random_draws[0]["purpose"],
        "summon_random_card_discard_cost"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0482_targeted_genesis_alt_payment_target_deals_damage() {
    let (checkpoint, source_id, avatar_id) = targeted_genesis_alt_payment_checkpoint();
    let mut session = checkpoint;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["paymentMode"] == "random-card-discard"
            && descriptor["genesisDamageTarget"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost"
        ]
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": source_id,
            "targetInstanceId": avatar_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert_exact_replay(&session);
}

fn targeted_genesis_sacrifice_manifest(seed: u32) -> String {
    let mut sacrificer = minion(2, 2);
    sacrificer["genesisMayDamageTargetAdjacentUnit"] = json!(2);
    sacrificer["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    sacrificer["manaCost"] = json!(2);
    let local = minion(0, 1);
    let mut warded_enemy = minion(1, 2);
    warded_enemy["summonToAnySite"] = json!(true);
    warded_enemy["ward"] = json!(true);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-sacrifice-targeted" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-sacrifice-targeted-v1",
        },
        "cards": {
            "north-avatar": avatar(false, 20),
            "north-local": local,
            "north-sacrificer": sacrificer,
            "north-site": site(),
            "south-avatar": avatar(false, 20),
            "south-minion": warded_enemy,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec![
                    "north-sacrificer",
                    "north-local",
                    "north-sacrificer",
                    "north-sacrificer",
                    "north-sacrificer",
                    "north-sacrificer",
                ],
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
    }))
}

fn targeted_genesis_sacrifice_checkpoint() -> (Session, String, String, String) {
    let manifest = targeted_genesis_sacrifice_manifest(499);
    let mut checkpoint = first_main(&manifest);
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-local"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut checkpoint, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before = state(&checkpoint);
    let source_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("sacrificer identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let local_id = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-local")
        .expect("local sacrifice minion")["instanceId"]
        .as_str()
        .expect("local identity")
        .to_owned();
    (checkpoint, source_id, avatar_id, local_id)
}

#[test]
fn rule_catalog_0499_targeted_genesis_sacrifice_decline_sacrifices_and_summons() {
    let (checkpoint, source_id, _, local_id) = targeted_genesis_sacrifice_checkpoint();
    let sacrifice_summons: Vec<_> = checkpoint
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardInstanceId"] == source_id
                && action.descriptor["cell"] == "C4"
                && action.descriptor["sacrificedMinionInstanceIds"] == json!([local_id.clone()])
        })
        .collect();
    assert!(
        sacrifice_summons
            .iter()
            .any(|action| action.descriptor["genesisDamageChoice"] == "decline")
    );
    let mut session = checkpoint;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["sacrificedMinionInstanceIds"] == json!([local_id])
            && descriptor["genesisDamageChoice"] == "decline"
    });
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert_eq!(receipt.events[2].payload["manaPaid"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0500_targeted_genesis_sacrifice_target_deals_damage() {
    let (checkpoint, source_id, avatar_id, local_id) = targeted_genesis_sacrifice_checkpoint();
    let mut session = checkpoint;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["sacrificedMinionInstanceIds"] == json!([local_id])
            && descriptor["genesisDamageTarget"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "minion-sacrificed",
            "minion-died",
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost"
        ]
    );
    assert_eq!(
        receipt.events[3].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": source_id,
            "targetInstanceId": avatar_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0501_spellbook_summon_draws_spell_then_heals_controller() {
    let mut scout = minion(1, 1);
    scout["genesisDrawSpells"] = json!(1);
    scout["genesisLoseControllerLife"] = json!(2);
    scout["genesisHealController"] = json!(2);
    scout["summonToAnySite"] = json!(true);
    scout["thresholds"] = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let manifest = scenario_manifest(501, &avatar(false, 20), &scout, &minion(1, 1), 8, 8, 8);
    let mut session = opening_checkpoint(&manifest);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "minion-summoned",
            "spell-drawn",
            "avatar-life-lost",
            "avatar-healed"
        ]
    );
    assert_eq!(receipt.events[2].payload["amount"], 2);
    assert_eq!(receipt.events[3].payload["amount"], 2);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0844_site_genesis_mana_pays_summon_and_expires_to_site_count() {
    let mut value = manifest_value(
        61,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisGainMana"] = json!(1);
    value["cards"]["north-minion"]["manaCost"] = json!(2);
    let manifest = finish_manifest(value);
    let mut session = Session::new(&manifest).expect("valid site-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let (_, played) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&played), ["site-played", "mana-gained"]);
    assert_eq!(
        played.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": played.events[0].payload["instanceId"].clone(),
        })
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["manaCost"] == 2
    });
    assert_eq!(state(&session)["players"]["north"]["mana"], 0);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    assert_eq!(state(&session)["players"]["north"]["mana"], 1);
    assert_exact_replay(&session);
}

fn site_heal_manifest(seed: u32) -> String {
    let mut loss = minion(1, 2);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(seed, &avatar(false, 20), &loss, &loss, 8, 8, 8);
    value["cards"]["north-site"]["genesisHealNearbyAvatars"] = json!(3);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0845_site_genesis_heal_targets_nearby_avatars_in_seat_order_and_caps() {
    let manifest = site_heal_manifest(62);
    let mut session = Session::new(&manifest).expect("valid site-heal Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    let (_, healed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C2"
    });
    assert_eq!(
        event_types(&healed),
        ["site-played", "avatar-healed", "avatar-healed"]
    );
    assert_eq!(
        healed
            .events
            .iter()
            .skip(1)
            .map(|event| event.payload["seat"].clone())
            .collect::<Vec<_>>(),
        [json!("north"), json!("south")]
    );
    for event in &healed.events[1..] {
        assert_eq!(event.payload["amount"], 2);
        assert_eq!(event.payload["attemptedAmount"], 3);
        assert_eq!(event.payload["life"], 20);
        assert_eq!(
            event.payload["sourceInstanceId"],
            healed.events[0].payload["instanceId"]
        );
    }
    assert_exact_replay(&session);
}

fn first_copy_mana_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        8,
        8,
        8,
    );
    for card_id in ["north-dark-site", "north-gothic-site"] {
        let mut facts = site();
        facts["genesisGainManaIfOnlyControlledCopy"] = json!(1);
        value["cards"]
            .as_object_mut()
            .expect("card definitions")
            .insert(card_id.to_owned(), facts);
    }
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["decks"]["north"]["atlas"] = json!([
        "north-dark-site",
        "north-gothic-site",
        "north-dark-site",
        "north-gothic-site",
        "north-dark-site",
        "north-gothic-site",
        "north-dark-site",
        "north-gothic-site",
    ]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0846_first_controlled_copy_site_genesis_mana_keys_by_card_id() {
    let manifest = first_copy_mana_manifest(67);
    let mut session = Session::new(&manifest).expect("valid first-copy site Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let (_, first_dark) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-dark-site"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&first_dark), ["site-played", "mana-gained"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, first_gothic) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-gothic-site"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&first_gothic), ["site-played", "mana-gained"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 3);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, second_dark) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-dark-site"
            && descriptor["cell"] == "B3"
    });
    assert_eq!(event_types(&second_dark), ["site-played"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0847_site_genesis_removes_only_enemy_stealth() {
    let mut stealth = minion(1, 2);
    stealth["stealth"] = json!(true);
    let mut value = manifest_value(71, &avatar(false, 20), &stealth, &stealth, 5, 5, 5);
    value["cards"]["north-site"]["genesisEnemiesLoseStealth"] = json!(true);
    let manifest = finish_manifest(value);
    let mut session = Session::new(&manifest).expect("valid Stealth-removal site Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (north_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let north_id = north_summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (south_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let south_id = south_summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["site-played", "stealth-lost"]);
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"].clone(),
        })
    );
    let after = state(&session);
    let units = after["realm"]["units"].as_array().expect("realm units");
    assert!(
        units
            .iter()
            .any(|unit| { unit["instanceId"] == north_id && unit["stealthed"] == true })
    );
    assert!(
        units
            .iter()
            .any(|unit| { unit["instanceId"] == south_id && unit["stealthed"] == false })
    );
    assert_exact_replay(&session);
}

fn adjacent_draw_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        6,
        5,
        5,
    );
    let mut matching_site = site();
    matching_site["genesisDrawSpellPerAdjacentSameCard"] = json!(true);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("matching-site".to_owned(), matching_site);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["matching-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["matching-site"; 6]);
    finish_manifest(value)
}

fn stacked_spell_genesis_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisDiscardTopSpells": 2,
        "genesisDrawSpellPerAdjacentSameCard": true,
    })
}

fn stacked_spell_genesis_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("stacked-site".to_owned(), stacked_spell_genesis_site());
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["stacked-site"; north_atlas_count]);
    value["decks"]["south"]["atlas"] = json!(vec!["stacked-site"; 6]);
    finish_manifest(value)
}

fn adjacent_combo_site(extra_facts: &Value) -> Value {
    let mut site = json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisDrawSpellPerAdjacentSameCard": true,
    });
    site.as_object_mut().expect("adjacent combo site").extend(
        extra_facts
            .as_object()
            .expect("extra Genesis facts")
            .clone(),
    );
    site
}

fn adjacent_combo_manifest(
    seed: u32,
    extra_facts: &Value,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "adjacent-combo-site".to_owned(),
            adjacent_combo_site(extra_facts),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; north_atlas_count]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

#[derive(Clone, Copy)]
enum AdjacentSetupGenesis {
    None,
    KeepNextSpell,
    KeepSpellOrder,
}

fn resolve_adjacent_setup_genesis(session: &mut Session, setup_genesis: AdjacentSetupGenesis) {
    match setup_genesis {
        AdjacentSetupGenesis::None => {}
        AdjacentSetupGenesis::KeepNextSpell => {
            accept_where(session, |descriptor| {
                descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
            });
        }
        AdjacentSetupGenesis::KeepSpellOrder => {
            accept_where(session, |descriptor| {
                descriptor["kind"] == "resolve-genesis-spell-order"
            });
        }
    }
}

fn reach_adjacent_combo_play(session: &mut Session, setup_genesis: AdjacentSetupGenesis) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    resolve_adjacent_setup_genesis(session, setup_genesis);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    resolve_adjacent_setup_genesis(session, setup_genesis);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn adjacent_token_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        6,
        north_spellbook_count,
        5,
    );
    let mut site_card = adjacent_combo_site(&json!({}));
    site_card["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("adjacent-combo-site".to_owned(), site_card);
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0848_adjacent_matching_site_genesis_draws_each_then_partially_decks_out() {
    let manifest = adjacent_draw_manifest(74);
    let mut session = Session::new(&manifest).expect("valid adjacent-draw site Genesis scenario");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (_, one_match) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&one_match), ["site-played", "spell-drawn"]);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    let before = state(&session);
    let last_spell = before["players"]["north"]["spellbook"][0].clone();
    let (_, two_matches) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    assert_eq!(
        event_types(&two_matches),
        ["site-played", "spell-drawn", "game-ended"]
    );
    let after = state(&session);
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["realm"]["sites"]["C2"]["cardId"], "matching-site");
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("Spellbook hand")
            .contains(&last_spell)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0406_site_genesis_discard_only_when_no_adjacent_same_card() {
    let manifest = stacked_spell_genesis_manifest(405, 5, 5);
    let mut session = Session::new(&manifest).expect("valid stacked site Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            receipt.events[0].payload["instanceId"]
        );
    }
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0405_site_genesis_draws_per_adjacent_then_discards_top_spells() {
    let manifest = stacked_spell_genesis_manifest(406, 6, 10);
    let mut session = Session::new(&manifest).expect("valid stacked site Genesis scenario");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before = state(&session);
    let spellbook = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook");
    let drawn_id = spellbook[0]["instanceId"].clone();
    let first_discard_id = spellbook[1]["instanceId"].clone();
    let second_discard_id = spellbook[2]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "spell-drawn",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_ne!(drawn_id, first_discard_id);
    assert_ne!(first_discard_id, second_discard_id);
    assert_eq!(receipt.events[2].payload["instanceId"], first_discard_id);
    assert_eq!(receipt.events[3].payload["instanceId"], second_discard_id);
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0849_site_genesis_publicly_discards_top_spells_without_deck_out() {
    for (seed, spellbook_count, discarded_count) in [(75, 5, 2), (76, 4, 1)] {
        let mut value = manifest_value(
            seed,
            &avatar(false, 20),
            &minion(1, 2),
            &minion(1, 2),
            5,
            spellbook_count,
            5,
        );
        value["cards"]["north-site"]["genesisDiscardTopSpells"] = json!(2);
        let manifest = finish_manifest(value);
        let mut session =
            Session::new(&manifest).expect("valid public-discard site Genesis scenario");
        keep(&mut session);
        keep(&mut session);
        let before = state(&session);
        let expected = before["players"]["north"]["spellbook"]
            .as_array()
            .expect("Spellbook")
            .clone();
        let (_, receipt) =
            accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        assert_eq!(receipt.events.len(), 1 + discarded_count);
        assert_eq!(receipt.events[0].event_type, "site-played");
        for (event, card) in receipt.events[1..].iter().zip(&expected) {
            assert_eq!(event.event_type, "spell-discarded");
            assert_eq!(event.payload["cardId"], card["cardId"]);
            assert_eq!(event.payload["instanceId"], card["instanceId"]);
            assert_eq!(event.payload["owner"], "north");
            assert_eq!(event.payload["seat"], "north");
            assert_eq!(
                event.payload["sourceInstanceId"],
                receipt.events[0].payload["instanceId"]
            );
        }
        let after = state(&session);
        assert_eq!(after["terminal"]["status"], "active");
        assert_eq!(
            after["players"]["north"]["spellbook"]
                .as_array()
                .map(Vec::len),
            Some(0)
        );
        assert_exact_replay(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario compares both engine-issued optional Genesis branches"
)]
fn rule_catalog_0857_optional_site_genesis_issues_decline_and_paid_token_branches() {
    let mut value = manifest_value(
        103,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    let manifest = finish_manifest(value);
    let mut checkpoint =
        Session::new(&manifest).expect("valid optional paid-token Genesis scenario");
    keep(&mut checkpoint);
    keep(&mut checkpoint);
    let origin = state(&checkpoint);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();

    let choices: Vec<_> = checkpoint
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cell"] == "C4"
                && action.descriptor["cardId"] == "north-site"
                && action.descriptor["cardInstanceId"] == source_instance_id
        })
        .collect();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["genesisTokenChoice"], "decline");
    assert_eq!(choices[0].label, "Play north-site at C4 (decline Genesis)");
    assert_eq!(choices[1].descriptor["genesisTokenChoice"], "pay-one-mana");
    assert_eq!(
        choices[1].label,
        "Play north-site at C4 (pay 1 for Genesis)"
    );
    assert_ne!(choices[0].action_id, choices[1].action_id);

    let mut declined = checkpoint.clone();
    let (_, declined_receipt) = accept_where(&mut declined, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(event_types(&declined_receipt), ["site-played"]);
    assert_eq!(state(&declined)["players"]["north"]["mana"], 1);
    assert_eq!(state(&declined)["realm"]["units"], json!([]));
    assert!(declined_receipt.random_draws.is_empty());
    assert_exact_replay(&declined);

    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let mut paid = checkpoint;
    let (_, paid_receipt) = accept_where(&mut paid, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&paid_receipt),
        ["site-played", "minion-summoned"]
    );
    assert!(paid_receipt.random_draws.is_empty());
    let paid_state = state(&paid);
    assert_eq!(paid_state["players"]["north"]["mana"], 0);
    assert_eq!(
        paid_receipt.events[1].payload,
        json!({
            "cardId": "foot-soldier",
            "cell": "C4",
            "instanceId": expected_token_id,
            "manaPaid": 1,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_instance_id,
            "token": true,
        })
    );
    assert_eq!(
        paid_state["realm"]["units"][0],
        json!({
            "cardId": "foot-soldier",
            "controller": "north",
            "damage": 0,
            "instanceId": expected_token_id,
            "location": "C4",
            "owner": "north",
            "region": "surface",
            "source": "token",
            "stealthed": false,
            "summoningSickness": true,
            "tapped": false,
            "warded": false,
        })
    );
    assert_exact_replay(&paid);
}

fn token_bottom_spell_genesis_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisMayBottomNextSpell"] = json!(true);
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    finish_manifest(value)
}

fn token_reorder_spell_genesis_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        6,
        5,
    );
    value["cards"]["north-site"]["genesisReorderNextSpells"] = json!(3);
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    finish_manifest(value)
}

fn token_discard_spell_genesis_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisDiscardTopSpells"] = json!(2);
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    finish_manifest(value)
}

fn token_and_mana_genesis_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisGainMana"] = json!(1);
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    finish_manifest(value)
}

#[test]
fn rule_catalog_0409_site_genesis_decline_token_still_grants_mana() {
    let manifest = token_and_mana_genesis_manifest(409);
    let mut session = Session::new(&manifest).expect("valid token-and-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    assert_eq!(state(&session)["realm"]["units"], json!([]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0410_site_genesis_pay_token_and_gain_mana_net() {
    let manifest = token_and_mana_genesis_manifest(410);
    let mut session = Session::new(&manifest).expect("valid token-and-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "mana-gained", "minion-summoned"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "cardId": "foot-soldier",
            "cell": "C4",
            "instanceId": expected_token_id,
            "manaPaid": 1,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_instance_id,
            "token": true,
        })
    );
    let final_state = state(&session);
    assert_eq!(final_state["players"]["north"]["mana"], 1);
    assert_eq!(
        final_state["realm"]["units"][0],
        json!({
            "cardId": "foot-soldier",
            "controller": "north",
            "damage": 0,
            "instanceId": expected_token_id,
            "location": "C4",
            "owner": "north",
            "region": "surface",
            "source": "token",
            "stealthed": false,
            "summoningSickness": true,
            "tapped": false,
            "warded": false,
        })
    );
    assert_exact_replay(&session);
}

fn mana_bottom_spell_genesis_facts() -> Value {
    json!({
        "genesisGainMana": 2,
        "genesisMayBottomNextSpell": true,
    })
}

fn mana_reorder_spell_genesis_facts() -> Value {
    json!({
        "genesisGainMana": 2,
        "genesisReorderNextSpells": 3,
    })
}

#[test]
fn rule_catalog_0411_site_genesis_gain_mana_then_bottom_next_spell() {
    let manifest = private_site_genesis_manifest(411, &mana_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        play_receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let choices = played.legal_actions().expect("private Genesis choices");
    assert_checkpoint_round_trip(&played);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["choice"], "bottom-next");
    assert_eq!(choices[1].descriptor["choice"], "keep-next");

    let mut bottomed = played;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let bottomed_state = state(&bottomed);
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);

    assert_eq!(bottomed_state["phase"], "main");
    assert_eq!(bottomed_state["stateVersion"], before_version + 2);
    assert_eq!(bottomed_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(bottomed_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        bottomed_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(bottomed_receipt.random_draws.is_empty());
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0412_site_genesis_gain_mana_then_decline_bottom_next_spell() {
    let manifest = private_site_genesis_manifest(412, &mana_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["players"]["north"]["mana"], mana_before + 3);

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let kept_state = state(&kept);

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(kept_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert_exact_replay(&kept);
}

fn mana_discard_spell_genesis_facts() -> Value {
    json!({
        "genesisDiscardTopSpells": 2,
        "genesisGainMana": 2,
    })
}

#[test]
fn rule_catalog_0415_site_genesis_gain_mana_then_discards_top_spells() {
    let manifest = private_site_genesis_manifest(415, &mana_discard_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let spellbook_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .len();
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (session, play, receipt) = play_private_genesis_site(&manifest);
    let after = state(&session);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "mana-gained",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    for (event, card) in receipt.events[2..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            receipt.events[0].payload["instanceId"]
        );
    }
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .map(Vec::len),
        Some(spellbook_before.saturating_sub(2))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0416_site_genesis_gain_mana_then_discards_only_available_spell() {
    let manifest = private_site_genesis_manifest(416, &mana_discard_spell_genesis_facts(), 4);
    let before = state(&opening_checkpoint(&manifest));
    let expected_discard = before["players"]["north"]["spellbook"][0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (session, play, receipt) = play_private_genesis_site(&manifest);
    let after = state(&session);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&receipt),
        ["site-played", "mana-gained", "spell-discarded"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(receipt.events[2].event_type, "spell-discarded");
    assert_eq!(
        receipt.events[2].payload["cardId"],
        expected_discard["cardId"]
    );
    assert_eq!(
        receipt.events[2].payload["instanceId"],
        expected_discard["instanceId"]
    );
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0417_site_genesis_pay_token_then_bottom_next_spell() {
    let manifest = token_bottom_spell_genesis_manifest(417);
    let mut session = Session::new(&manifest).expect("valid token-and-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = before["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let before_version = before["stateVersion"].as_u64().expect("state version");

    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    let played_state = state(&session);

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "minion-summoned"]
    );
    assert_eq!(
        play_receipt.events[1].payload,
        json!({
            "cardId": "foot-soldier",
            "cell": "C4",
            "instanceId": expected_token_id,
            "manaPaid": 1,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_instance_id,
            "token": true,
        })
    );
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], 0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(
        played_state["realm"]["units"][0]["instanceId"],
        json!(expected_token_id)
    );

    let choices = session.legal_actions().expect("private Genesis choices");
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["choice"], "bottom-next");
    assert_eq!(choices[1].descriptor["choice"], "keep-next");

    let (_, bottomed_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let bottomed_state = state(&session);
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);

    assert_eq!(bottomed_state["phase"], "main");
    assert_eq!(bottomed_state["stateVersion"], before_version + 2);
    assert_eq!(bottomed_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(bottomed_state["players"]["north"]["mana"], 0);
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        bottomed_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(bottomed_receipt.random_draws.is_empty());
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0418_site_genesis_decline_token_then_keep_next_spell() {
    let manifest = token_bottom_spell_genesis_manifest(418);
    let mut session = Session::new(&manifest).expect("valid token-and-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let before_version = before["stateVersion"].as_u64().expect("state version");

    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    let played_state = state(&session);

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], 1);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(played_state["realm"]["units"], json!([]));
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let (_, kept_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let kept_state = state(&session);

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(kept_state["players"]["north"]["mana"], 1);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0419_site_genesis_decline_token_then_discards_top_spells() {
    let manifest = token_discard_spell_genesis_manifest(419);
    let mut session = Session::new(&manifest).expect("valid token-and-discard Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let spellbook_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .len();
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    let after = state(&session);

    assert_eq!(
        event_types(&receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            receipt.events[0].payload["instanceId"]
        );
    }
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["north"]["mana"], 1);
    assert_eq!(after["realm"]["units"], json!([]));
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .map(Vec::len),
        Some(spellbook_before.saturating_sub(2))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0420_site_genesis_pay_token_then_discards_top_spells() {
    let manifest = token_discard_spell_genesis_manifest(420);
    let mut session = Session::new(&manifest).expect("valid token-and-discard Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let spellbook_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .len();
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = before["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    let after = state(&session);

    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "minion-summoned",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cardId": "foot-soldier",
            "cell": "C4",
            "instanceId": expected_token_id,
            "manaPaid": 1,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_instance_id,
            "token": true,
        })
    );
    for (event, card) in receipt.events[2..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            receipt.events[0].payload["instanceId"]
        );
    }
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["north"]["mana"], 0);
    assert_eq!(
        after["realm"]["units"][0]["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .map(Vec::len),
        Some(spellbook_before.saturating_sub(2))
    );
    assert_exact_replay(&session);
}

fn bottom_reorder_spell_genesis_facts() -> Value {
    json!({
        "genesisMayBottomNextSpell": true,
        "genesisReorderNextSpells": 3,
    })
}

#[test]
fn rule_catalog_0421_site_genesis_bottom_then_reorder_next_spells() {
    let manifest = private_site_genesis_manifest(421, &bottom_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(played_state["pendingGenesisSpellOrder"], Value::Null);

    let mut bottomed = played;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let bottomed_state = state(&bottomed);
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);

    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(bottomed_state["phase"], "genesis");
    assert_eq!(bottomed_state["stateVersion"], before_version + 2);
    assert_eq!(bottomed_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated.clone())
    );
    assert_eq!(
        bottomed_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut reordered = bottomed;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let reordered_state = state(&reordered);
    let before_cards = rotated;

    assert_eq!(reordered_state["phase"], "main");
    assert_eq!(reordered_state["stateVersion"], before_version + 3);
    assert_eq!(reordered_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][0],
        before_cards[2]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][1],
        before_cards[1]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][2],
        before_cards[0]
    );
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        reordered_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0422_site_genesis_keep_then_reorder_next_spells() {
    let manifest = private_site_genesis_manifest(422, &bottom_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let kept_state = state(&kept);

    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(kept_state["phase"], "genesis");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        kept_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut reordered = kept;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let reordered_state = state(&reordered);

    assert_eq!(reordered_state["phase"], "main");
    assert_eq!(reordered_state["stateVersion"], before_version + 3);
    assert_eq!(reordered_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        reordered_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_exact_replay(&reordered);
}

fn discard_bottom_spell_genesis_facts() -> Value {
    json!({
        "genesisDiscardTopSpells": 2,
        "genesisMayBottomNextSpell": true,
    })
}

#[test]
fn rule_catalog_0425_site_genesis_discard_then_bottom_next_spell() {
    let manifest = private_site_genesis_manifest(425, &discard_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let expected_discards = before_spellbook
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let top_after_discard = before_spellbook[2].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in play_receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            play_receipt.events[0].payload["instanceId"]
        );
    }
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    let mut after_discard = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    after_discard.remove(0);
    after_discard.remove(0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        Value::Array(after_discard.clone())
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let choices = played.legal_actions().expect("private Genesis choices");
    assert_checkpoint_round_trip(&played);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["choice"], "bottom-next");
    assert_eq!(choices[1].descriptor["choice"], "keep-next");

    let mut bottomed = played;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let bottomed_state = state(&bottomed);
    let mut rotated = after_discard.clone();
    let first = rotated.remove(0);
    rotated.push(first);

    assert_eq!(bottomed_state["phase"], "main");
    assert_eq!(bottomed_state["stateVersion"], before_version + 2);
    assert_eq!(bottomed_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        bottomed_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(bottomed_receipt.random_draws.is_empty());
    let hidden_top_id = top_after_discard["instanceId"]
        .as_str()
        .expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0426_site_genesis_discard_then_keep_next_spell() {
    let manifest = private_site_genesis_manifest(426, &discard_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let expected_discards = before_spellbook
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in play_receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
    }
    assert_eq!(played_state["phase"], "genesis");
    let mut after_discard = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    after_discard.remove(0);
    after_discard.remove(0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        Value::Array(after_discard.clone())
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let kept_state = state(&kept);

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        Value::Array(after_discard)
    );
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert_exact_replay(&kept);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario compares paid-token and hidden reorder Genesis"
)]
fn rule_catalog_0423_site_genesis_pay_token_then_reorder_next_spells() {
    let manifest = token_reorder_spell_genesis_manifest(423);
    let mut session = Session::new(&manifest).expect("valid token-and-reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = before["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let before_version = before["stateVersion"].as_u64().expect("state version");

    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    let played_state = state(&session);

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "minion-summoned"]
    );
    assert_eq!(
        play_receipt.events[1].payload,
        json!({
            "cardId": "foot-soldier",
            "cell": "C4",
            "instanceId": expected_token_id,
            "manaPaid": 1,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_instance_id,
            "token": true,
        })
    );
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], 0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(played_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(
        played_state["realm"]["units"][0]["instanceId"],
        json!(expected_token_id)
    );

    let (_, reordered_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let reordered_state = state(&session);
    let before_cards = before_spellbook.as_array().expect("Spellbook cards");

    assert_eq!(reordered_state["phase"], "main");
    assert_eq!(reordered_state["stateVersion"], before_version + 2);
    assert_eq!(reordered_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(reordered_state["players"]["north"]["mana"], 0);
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][0],
        before_cards[2]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][1],
        before_cards[1]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][2],
        before_cards[0]
    );
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        reordered_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0424_site_genesis_decline_token_then_reorder_next_spells() {
    let manifest = token_reorder_spell_genesis_manifest(424);
    let mut session = Session::new(&manifest).expect("valid token-and-reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let before_version = before["stateVersion"].as_u64().expect("state version");

    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    let played_state = state(&session);

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], 1);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(played_state["realm"]["units"], json!([]));
    assert_eq!(played_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let (_, reordered_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let reordered_state = state(&session);

    assert_eq!(reordered_state["phase"], "main");
    assert_eq!(reordered_state["stateVersion"], before_version + 2);
    assert_eq!(reordered_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(reordered_state["players"]["north"]["mana"], 1);
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        reordered_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(reordered_receipt.random_draws.is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0413_site_genesis_gain_mana_then_reorder_next_spells() {
    let manifest = private_site_genesis_manifest(413, &mana_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        play_receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(played_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let choices = played.legal_actions().expect("private reorder choices");
    assert_checkpoint_round_trip(&played);
    assert_eq!(choices.len(), 6);
    assert!(
        choices
            .iter()
            .all(|choice| { choice.descriptor["kind"] == "resolve-genesis-spell-order" })
    );

    let mut reversed = played;
    let (_, reversed_receipt) = accept_where(&mut reversed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let reversed_state = state(&reversed);
    let before_cards = before_spellbook.as_array().expect("Spellbook cards");

    assert_eq!(reversed_state["phase"], "main");
    assert_eq!(reversed_state["stateVersion"], before_version + 2);
    assert_eq!(reversed_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(reversed_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][0],
        before_cards[2]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][1],
        before_cards[1]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][2],
        before_cards[0]
    );
    assert_eq!(event_types(&reversed_receipt), ["spells-reordered"]);
    assert_eq!(
        reversed_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(reversed_receipt.random_draws.is_empty());
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reversed_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reversed);
}

#[test]
fn rule_catalog_0414_site_genesis_gain_mana_then_keep_spell_order() {
    let manifest = private_site_genesis_manifest(414, &mana_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["players"]["north"]["mana"], mana_before + 3);

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let kept_state = state(&kept);

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(kept_state["players"]["north"]["mana"], mana_before + 3);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert_exact_replay(&kept);
}

fn discard_reorder_spell_genesis_facts() -> Value {
    json!({
        "genesisDiscardTopSpells": 2,
        "genesisReorderNextSpells": 3,
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one discard-then-reorder Genesis proof"
)]
fn rule_catalog_0427_site_genesis_discard_then_reorder_next_spells() {
    let manifest = private_site_genesis_manifest(427, &discard_reorder_spell_genesis_facts(), 8);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let expected_discards = before_spellbook
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let top_after_discard = before_spellbook[2].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in play_receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(
            event.payload["sourceInstanceId"],
            play_receipt.events[0].payload["instanceId"]
        );
    }
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    let mut after_discard = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    after_discard.remove(0);
    after_discard.remove(0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        Value::Array(after_discard.clone())
    );
    assert_eq!(played_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let choices = played.legal_actions().expect("private reorder choices");
    assert_checkpoint_round_trip(&played);
    assert_eq!(choices.len(), 6);
    assert!(
        choices
            .iter()
            .all(|choice| choice.descriptor["kind"] == "resolve-genesis-spell-order")
    );

    let mut reversed = played;
    let (_, reversed_receipt) = accept_where(&mut reversed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let reversed_state = state(&reversed);
    let after_discard_cards = after_discard;

    assert_eq!(reversed_state["phase"], "main");
    assert_eq!(reversed_state["stateVersion"], before_version + 2);
    assert_eq!(reversed_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][0],
        after_discard_cards[2]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][1],
        after_discard_cards[1]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][2],
        after_discard_cards[0]
    );
    assert_eq!(event_types(&reversed_receipt), ["spells-reordered"]);
    assert_eq!(
        reversed_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(reversed_receipt.random_draws.is_empty());
    let hidden_top_id = top_after_discard["instanceId"]
        .as_str()
        .expect("top instance ID");
    assert!(
        !serde_json::to_string(&reversed_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reversed);
}

#[test]
fn rule_catalog_0428_site_genesis_discard_then_keep_spell_order() {
    let manifest = private_site_genesis_manifest(428, &discard_reorder_spell_genesis_facts(), 8);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let expected_discards = before_spellbook
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "spell-discarded", "spell-discarded"]
    );
    for (event, card) in play_receipt.events[1..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
        assert_eq!(event.payload["instanceId"], card["instanceId"]);
    }
    assert_eq!(played_state["phase"], "genesis");
    let mut after_discard = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    after_discard.remove(0);
    after_discard.remove(0);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        Value::Array(after_discard.clone())
    );
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let kept_state = state(&kept);

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        Value::Array(after_discard)
    );
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0429_site_genesis_gain_mana_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(429, &json!({ "genesisGainMana": 2 }), 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent gain-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "mana-gained", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    assert_eq!(
        receipt.events[2].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 3
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0430_site_genesis_gain_mana_only_without_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(430, &json!({ "genesisGainMana": 2 }), 5, 5);
    let mut session = Session::new(&manifest).expect("valid adjacent gain-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 3
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0431_site_genesis_draw_per_adjacent_then_bottom_next_spell() {
    let manifest =
        adjacent_combo_manifest(431, &json!({ "genesisMayBottomNextSpell": true }), 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent may-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepNextSpell);
    let before = state(&session);
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (play, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let source_instance_id = play["cardInstanceId"].clone();
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let played_state = state(&session);
    let after_draw_spellbook = played_state["players"]["north"]["spellbook"].clone();
    let top = after_draw_spellbook[0].clone();
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(
        played_state["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut bottomed = session;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let bottomed_state = state(&bottomed);
    let mut rotated = after_draw_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(bottomed_state["phase"], "main");
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0432_site_genesis_draw_per_adjacent_then_keep_next_spell() {
    let manifest =
        adjacent_combo_manifest(432, &json!({ "genesisMayBottomNextSpell": true }), 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent may-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepNextSpell);
    let (play, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let source_instance_id = play["cardInstanceId"].clone();
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let after_draw_spellbook = state(&session)["players"]["north"]["spellbook"].clone();
    assert_eq!(state(&session)["phase"], "genesis");
    assert_eq!(
        state(&session)["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let kept_state = state(&kept);
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(kept_state["phase"], "main");
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        after_draw_spellbook
    );
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0433_site_genesis_draw_per_adjacent_then_reorder_next_spells() {
    let manifest = adjacent_combo_manifest(433, &json!({ "genesisReorderNextSpells": 3 }), 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepSpellOrder);
    let (play, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let source_instance_id = play["cardInstanceId"].clone();
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let played_state = state(&session);
    let after_draw_spellbook = played_state["players"]["north"]["spellbook"].clone();
    let top = after_draw_spellbook[0].clone();
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut reordered = session;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let reordered_state = state(&reordered);
    assert_eq!(reordered_state["phase"], "main");
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][0],
        after_draw_spellbook[2]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][1],
        after_draw_spellbook[1]
    );
    assert_eq!(
        reordered_state["players"]["north"]["spellbook"][2],
        after_draw_spellbook[0]
    );
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0434_site_genesis_draw_per_adjacent_then_keep_spell_order() {
    let manifest = adjacent_combo_manifest(434, &json!({ "genesisReorderNextSpells": 3 }), 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepSpellOrder);
    let before_final = state(&session);
    let mut expected_after_draw = before_final["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let (play, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let source_instance_id = play["cardInstanceId"].clone();
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    expected_after_draw.remove(0);
    assert_eq!(
        state(&session)["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let kept_state = state(&kept);
    assert_eq!(kept_state["phase"], "main");
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        Value::Array(expected_after_draw)
    );
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0435_site_genesis_pay_token_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_token_manifest(435, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent token Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let origin_state_version = before["stateVersion"].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C3",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "minion-summoned", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(
        receipt.events[2].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0436_site_genesis_decline_token_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_token_manifest(436, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent token Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(event_types(&receipt), ["site-played", "spell-drawn"]);
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    let after = state(&session);
    assert_eq!(after["realm"]["units"], json!([]));
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

fn adjacent_stealth_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut stealth = minion(1, 2);
    stealth["stealth"] = json!(true);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &stealth,
        &stealth,
        6,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "adjacent-combo-site".to_owned(),
            adjacent_combo_site(&json!({ "genesisEnemiesLoseStealth": true })),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

fn setup_adjacent_stealth_minions(session: &mut Session) -> Value {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (north_summon, _) =
        accept_where(session, |descriptor| descriptor["kind"] == "summon-minion");
    let north_id = north_summon["cardInstanceId"].clone();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (south_summon, _) =
        accept_where(session, |descriptor| descriptor["kind"] == "summon-minion");
    let south_id = south_summon["cardInstanceId"].clone();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert_ne!(north_id, south_id);
    south_id
}

fn adjacent_heal_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut loss = minion(1, 2);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &loss,
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "adjacent-combo-site".to_owned(),
            adjacent_combo_site(&json!({ "genesisHealNearbyAvatars": 3 })),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; north_atlas_count]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0437_site_genesis_strip_stealth_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_stealth_manifest(437, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "stealth-lost", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["controller"] == "south" && unit["stealthed"] == false)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0438_site_genesis_strip_stealth_only_without_adjacent_same_card() {
    let manifest = adjacent_stealth_manifest(438, 6);
    let mut session = Session::new(&manifest).expect("valid adjacent stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played"]);
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_eq!(after["realm"]["units"], json!([]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0439_site_genesis_heal_nearby_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_heal_manifest(439, 6, 10);
    let mut session = Session::new(&manifest).expect("valid adjacent heal Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
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
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "avatar-healed", "spell-drawn"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["attemptedAmount"], 3);
    assert_eq!(receipt.events[1].payload["life"], 20);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0440_site_genesis_heal_nearby_only_without_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(440, &json!({ "genesisHealNearbyAvatars": 3 }), 6, 5);
    let mut session = Session::new(&manifest).expect("valid adjacent heal Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played"]);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0441_site_genesis_immobilize_nearby_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(
        441,
        &json!({ "genesisImmobilizeNearbyUntilNextTurn": true }),
        6,
        10,
    );
    let mut session = Session::new(&manifest).expect("valid adjacent immobilize Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["site-played", "spell-drawn"]);
    let after = state(&session);
    let north_area = after["realm"]["immobileAreas"]
        .as_array()
        .expect("immobile areas")
        .iter()
        .find(|area| {
            area["expiresAtSeat"] == "north"
                && area["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.contains(&json!("C3")))
        })
        .cloned()
        .expect("north immobile area at C3");
    assert_eq!(north_area["cells"], json!(["C3", "C4"]));
    assert_eq!(
        north_area["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0442_site_genesis_immobilize_nearby_only_without_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(
        442,
        &json!({ "genesisImmobilizeNearbyUntilNextTurn": true }),
        5,
        5,
    );
    let mut session = Session::new(&manifest).expect("valid adjacent immobilize Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played"]);
    assert_eq!(
        state(&session)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        }])
    );
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

fn token_stealth_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut stealth = minion(1, 2);
    stealth["stealth"] = json!(true);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &stealth,
        &stealth,
        6,
        north_spellbook_count,
        5,
    );
    let mut site_card = adjacent_combo_site(&json!({ "genesisEnemiesLoseStealth": true }));
    site_card["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("adjacent-combo-site".to_owned(), site_card);
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

fn heal_bottom_site() -> Value {
    let mut card = site();
    card.as_object_mut().expect("heal-bottom site").extend(
        json!({
            "genesisHealNearbyAvatars": 3,
            "genesisMayBottomNextSpell": true,
        })
        .as_object()
        .expect("heal-bottom facts")
        .clone(),
    );
    card
}

fn adjacent_heal_bottom_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut loss = minion(1, 2);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &loss,
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]["setup-site"] = site();
    value["cards"]["heal-bottom-site"] = heal_bottom_site();
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    let mut north_atlas = vec!["setup-site"; north_atlas_count.saturating_sub(2)];
    north_atlas.extend(["heal-bottom-site", "heal-bottom-site"]);
    value["decks"]["north"]["atlas"] = json!(north_atlas);
    value["decks"]["south"]["atlas"] = json!(vec!["setup-site"; 6]);
    finish_manifest(value)
}

fn reach_adjacent_heal_play(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardId"] == "setup-site"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "summon-minion");
    assert_eq!(state(session)["players"]["north"]["avatar"]["life"], 18);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C1"
            && descriptor["cardId"] == "setup-site"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn immobilize_reorder_spell_genesis_facts() -> Value {
    json!({
        "genesisImmobilizeNearbyUntilNextTurn": true,
        "genesisReorderNextSpells": 3,
    })
}

#[test]
fn rule_catalog_0443_site_genesis_pay_token_then_strip_stealth_per_adjacent() {
    let manifest = token_stealth_manifest(443, 10);
    let mut session = Session::new(&manifest).expect("valid token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C3",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "minion-summoned",
            "stealth-lost",
            "spell-drawn"
        ]
    );
    assert_eq!(
        receipt.events[1].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0444_site_genesis_decline_token_then_strip_stealth_per_adjacent() {
    let manifest = token_stealth_manifest(444, 10);
    let mut session = Session::new(&manifest).expect("valid token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "stealth-lost", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["controller"] == "south" && unit["stealthed"] == false)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0445_site_genesis_heal_nearby_then_bottom_next_spell() {
    let manifest = adjacent_heal_bottom_manifest(445, 6, 10);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mut session = Session::new(&manifest).expect("valid adjacent heal-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_heal_play(&mut session);
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardId"] == "heal-bottom-site"
    });

    assert_eq!(event_types(&play_receipt), ["site-played", "avatar-healed"]);
    assert_eq!(play_receipt.events[1].payload["amount"], 2);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["phase"], "genesis");

    let mut bottomed = session;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        state(&bottomed)["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0446_site_genesis_heal_nearby_then_keep_next_spell() {
    let manifest = adjacent_heal_bottom_manifest(446, 6, 10);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mut session = Session::new(&manifest).expect("valid adjacent heal-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_heal_play(&mut session);
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardId"] == "heal-bottom-site"
    });

    assert_eq!(event_types(&play_receipt), ["site-played", "avatar-healed"]);
    assert_eq!(state(&session)["phase"], "genesis");

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(state(&kept)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0447_site_genesis_immobilize_nearby_then_reorder_next_spells() {
    let manifest = private_site_genesis_manifest(447, &immobilize_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(
        state(&played)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": source_instance_id,
        }])
    );
    assert_eq!(state(&played)["phase"], "genesis");

    let mut reordered = played;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&reordered)["players"]["north"]["spellbook"][0],
        before_spellbook[2]
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0448_site_genesis_immobilize_nearby_then_keep_spell_order() {
    let manifest = private_site_genesis_manifest(448, &immobilize_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert_eq!(
        state(&played)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        }])
    );

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_exact_replay(&kept);
}

fn conditional_mana_bottom_spell_genesis_facts() -> Value {
    json!({
        "genesisGainManaIfOnlyControlledCopy": 1,
        "genesisMayBottomNextSpell": true,
    })
}

fn conditional_mana_reorder_spell_genesis_facts() -> Value {
    json!({
        "genesisGainManaIfOnlyControlledCopy": 1,
        "genesisReorderNextSpells": 3,
    })
}

fn conditional_token_and_mana_genesis_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 1),
        &minion(1, 1),
        5,
        5,
        5,
    );
    value["cards"]["north-site"]["genesisGainManaIfOnlyControlledCopy"] = json!(1);
    value["cards"]["north-site"]["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    finish_manifest(value)
}

fn conditional_mana_discard_spell_genesis_facts() -> Value {
    json!({
        "genesisDiscardTopSpells": 2,
        "genesisGainManaIfOnlyControlledCopy": 1,
    })
}

fn single_copy_conditional_mana_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        8,
        8,
        8,
    );
    let mut conditional = site();
    conditional["genesisGainManaIfOnlyControlledCopy"] = json!(1);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("conditional-site".to_owned(), conditional);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("north-filler-site".to_owned(), site());
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["decks"]["north"]["atlas"] = json!([
        "conditional-site",
        "north-filler-site",
        "conditional-site",
        "north-filler-site",
        "conditional-site",
        "north-filler-site",
        "conditional-site",
        "north-filler-site",
    ]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0449_site_genesis_conditional_mana_then_discards_top_spells() {
    let manifest =
        private_site_genesis_manifest(449, &conditional_mana_discard_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (session, play, receipt) = play_private_genesis_site(&manifest);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "mana-gained",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    for (event, card) in receipt.events[2..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
    }
    assert_eq!(
        state(&session)["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0450_site_genesis_conditional_mana_only_on_first_controlled_copy() {
    let manifest = single_copy_conditional_mana_manifest(67);
    let mut session =
        Session::new(&manifest).expect("valid single-copy conditional-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let (_, first_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "conditional-site"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&first_play), ["site-played", "mana-gained"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, filler_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-filler-site"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&filler_play), ["site-played"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, second_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "conditional-site"
            && descriptor["cell"] == "B3"
    });
    assert_eq!(event_types(&second_play), ["site-played"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0451_site_genesis_conditional_mana_then_bottom_next_spell() {
    let manifest =
        private_site_genesis_manifest(451, &conditional_mana_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        play_receipt.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(state(&played)["phase"], "genesis");
    assert_eq!(state(&played)["players"]["north"]["mana"], mana_before + 2);

    let mut bottomed = played;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        state(&bottomed)["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0452_site_genesis_conditional_mana_then_keep_next_spell() {
    let manifest =
        private_site_genesis_manifest(452, &conditional_mana_bottom_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(state(&played)["phase"], "genesis");
    assert_eq!(state(&played)["players"]["north"]["mana"], mana_before + 2);

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0453_site_genesis_conditional_mana_then_reorder_next_spells() {
    let manifest =
        private_site_genesis_manifest(453, &conditional_mana_reorder_spell_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, _, play_receipt) = play_private_genesis_site(&manifest);

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(play_receipt.events[1].payload["amount"], 1);
    assert_eq!(state(&played)["phase"], "genesis");
    assert_eq!(state(&played)["players"]["north"]["mana"], mana_before + 2);

    let mut reordered = played;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&reordered)["players"]["north"]["spellbook"][0],
        before_spellbook[2]
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0454_site_genesis_conditional_mana_and_pay_token_net() {
    let manifest = conditional_token_and_mana_genesis_manifest(454);
    let mut session =
        Session::new(&manifest).expect("valid conditional token-and-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "mana-gained", "minion-summoned"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(
        receipt.events[2].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 1);
    assert_exact_replay(&session);
}

fn conditional_mana_fact() -> Value {
    json!({ "genesisGainManaIfOnlyControlledCopy": 1 })
}

fn adjacent_stealth_conditional_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut stealth = minion(1, 2);
    stealth["stealth"] = json!(true);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &stealth,
        &stealth,
        6,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "adjacent-combo-site".to_owned(),
            adjacent_combo_site(&json!({
                "genesisEnemiesLoseStealth": true,
                "genesisGainManaIfOnlyControlledCopy": 1,
            })),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

fn adjacent_heal_conditional_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut loss = minion(1, 2);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &loss,
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "adjacent-combo-site".to_owned(),
            adjacent_combo_site(&json!({
                "genesisHealNearbyAvatars": 3,
                "genesisGainManaIfOnlyControlledCopy": 1,
            })),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; north_atlas_count]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0455_site_genesis_conditional_mana_then_draw_per_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(455, &conditional_mana_fact(), 6, 10);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["site-played", "spell-drawn"]);
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 1
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0456_site_genesis_conditional_mana_only_without_adjacent_same_card() {
    let manifest = adjacent_combo_manifest(456, &conditional_mana_fact(), 5, 5);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0457_site_genesis_conditional_mana_strip_stealth_then_draw_per_adjacent() {
    let manifest = adjacent_stealth_conditional_manifest(457, 10);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "stealth-lost", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["controller"] == "south" && unit["stealthed"] == false)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0458_site_genesis_conditional_mana_strip_stealth_only_without_adjacent() {
    let manifest = adjacent_stealth_conditional_manifest(458, 6);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_eq!(after["realm"]["units"], json!([]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0459_site_genesis_conditional_mana_heal_nearby_then_draw_per_adjacent() {
    let manifest = adjacent_heal_conditional_manifest(459, 6, 10);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional heal Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
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
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "avatar-healed", "spell-drawn"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["attemptedAmount"], 3);
    assert_eq!(receipt.events[1].payload["life"], 20);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0460_site_genesis_conditional_mana_heal_nearby_only_without_adjacent() {
    let manifest = adjacent_combo_manifest(
        460,
        &json!({
            "genesisHealNearbyAvatars": 3,
            "genesisGainManaIfOnlyControlledCopy": 1,
        }),
        6,
        5,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional heal Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(
        after["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0461_site_genesis_conditional_mana_immobilize_nearby_then_draw_per_adjacent() {
    let manifest = adjacent_combo_manifest(
        461,
        &json!({
            "genesisImmobilizeNearbyUntilNextTurn": true,
            "genesisGainManaIfOnlyControlledCopy": 1,
        }),
        6,
        10,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional immobilize Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["site-played", "spell-drawn"]);
    let after = state(&session);
    let north_area = after["realm"]["immobileAreas"]
        .as_array()
        .expect("immobile areas")
        .iter()
        .find(|area| {
            area["expiresAtSeat"] == "north"
                && area["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.contains(&json!("C3")))
        })
        .cloned()
        .expect("north immobile area at C3");
    assert_eq!(north_area["cells"], json!(["C3", "C4"]));
    assert_eq!(
        north_area["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0462_site_genesis_conditional_mana_immobilize_nearby_only_without_adjacent() {
    let manifest = adjacent_combo_manifest(
        462,
        &json!({
            "genesisImmobilizeNearbyUntilNextTurn": true,
            "genesisGainManaIfOnlyControlledCopy": 1,
        }),
        5,
        5,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional immobilize Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(
        state(&session)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        }])
    );
    assert_eq!(
        state(&session)["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

fn adjacent_conditional_token_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        6,
        north_spellbook_count,
        5,
    );
    let mut site_card = adjacent_combo_site(&json!({
        "genesisGainManaIfOnlyControlledCopy": 1,
    }));
    site_card["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("adjacent-combo-site".to_owned(), site_card);
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

fn stacked_conditional_spell_genesis_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisDiscardTopSpells": 2,
        "genesisDrawSpellPerAdjacentSameCard": true,
        "genesisGainManaIfOnlyControlledCopy": 1,
    })
}

fn stacked_conditional_spell_genesis_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert(
            "stacked-site".to_owned(),
            stacked_conditional_spell_genesis_site(),
        );
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["stacked-site"; north_atlas_count]);
    value["decks"]["south"]["atlas"] = json!(vec!["stacked-site"; 6]);
    finish_manifest(value)
}

fn reach_stacked_adjacent_play(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

#[test]
fn rule_catalog_0463_site_genesis_conditional_mana_pay_token_then_draw_per_adjacent() {
    let manifest = adjacent_conditional_token_manifest(463, 10);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional token Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let origin_state_version = before["stateVersion"].clone();
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C3",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "minion-summoned", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(
        receipt.events[2].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0464_site_genesis_conditional_mana_decline_token_then_draw_per_adjacent() {
    let manifest = adjacent_conditional_token_manifest(464, 10);
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional token Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::None);
    let before = state(&session);
    let source_instance_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(event_types(&receipt), ["site-played", "spell-drawn"]);
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_eq!(state(&session)["realm"]["units"], json!([]));
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0465_site_genesis_conditional_mana_draw_per_adjacent_then_bottom_next_spell() {
    let manifest = adjacent_combo_manifest(
        465,
        &json!({
            "genesisGainManaIfOnlyControlledCopy": 1,
            "genesisMayBottomNextSpell": true,
        }),
        6,
        10,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional may-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepNextSpell);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let after_draw_spellbook = state(&session)["players"]["north"]["spellbook"].clone();
    let top = after_draw_spellbook[0].clone();
    assert_eq!(state(&session)["phase"], "genesis");
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );

    let mut bottomed = session;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let mut rotated = after_draw_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        state(&bottomed)["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_eq!(state(&bottomed)["phase"], "main");
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0466_site_genesis_conditional_mana_draw_per_adjacent_then_keep_next_spell() {
    let manifest = adjacent_combo_manifest(
        466,
        &json!({
            "genesisGainManaIfOnlyControlledCopy": 1,
            "genesisMayBottomNextSpell": true,
        }),
        6,
        10,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional may-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepNextSpell);
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let after_draw_spellbook = state(&session)["players"]["north"]["spellbook"].clone();
    assert_eq!(state(&session)["phase"], "genesis");

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(state(&kept)["phase"], "main");
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        after_draw_spellbook
    );
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0467_site_genesis_conditional_mana_draw_per_adjacent_then_reorder_next_spells() {
    let manifest = adjacent_combo_manifest(
        467,
        &json!({
            "genesisGainManaIfOnlyControlledCopy": 1,
            "genesisReorderNextSpells": 3,
        }),
        6,
        10,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepSpellOrder);
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    let after_draw_spellbook = state(&session)["players"]["north"]["spellbook"].clone();
    let top = after_draw_spellbook[0].clone();
    assert_eq!(state(&session)["phase"], "genesis");

    let mut reordered = session;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&reordered)["players"]["north"]["spellbook"][0],
        after_draw_spellbook[2]
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_eq!(state(&reordered)["phase"], "main");
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0468_site_genesis_conditional_mana_draw_per_adjacent_then_keep_spell_order() {
    let manifest = adjacent_combo_manifest(
        468,
        &json!({
            "genesisGainManaIfOnlyControlledCopy": 1,
            "genesisReorderNextSpells": 3,
        }),
        6,
        10,
    );
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional reorder Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_combo_play(&mut session, AdjacentSetupGenesis::KeepSpellOrder);
    let before_final = state(&session);
    let mut expected_after_draw = before_final["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&play_receipt), ["site-played", "spell-drawn"]);
    expected_after_draw.remove(0);

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_eq!(state(&kept)["phase"], "main");
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        Value::Array(expected_after_draw)
    );
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0469_site_genesis_conditional_mana_draws_per_adjacent_then_discards_top_spells() {
    let manifest = stacked_conditional_spell_genesis_manifest(469, 6, 10);
    let mut session =
        Session::new(&manifest).expect("valid stacked conditional site Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_stacked_adjacent_play(&mut session);
    let before = state(&session);
    let spellbook = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook");
    let drawn_id = spellbook[0]["instanceId"].clone();
    let first_discard_id = spellbook[1]["instanceId"].clone();
    let second_discard_id = spellbook[2]["instanceId"].clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "spell-drawn",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_ne!(drawn_id, first_discard_id);
    assert_ne!(first_discard_id, second_discard_id);
    assert_eq!(receipt.events[2].payload["instanceId"], first_discard_id);
    assert_eq!(receipt.events[3].payload["instanceId"], second_discard_id);
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0470_site_genesis_conditional_mana_discards_top_spells_without_adjacent_match() {
    let manifest = stacked_conditional_spell_genesis_manifest(470, 5, 5);
    let mut session =
        Session::new(&manifest).expect("valid stacked conditional site Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let expected_discards = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("Spellbook")
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "mana-gained",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    for (event, card) in receipt.events[2..].iter().zip(&expected_discards) {
        assert_eq!(event.event_type, "spell-discarded");
        assert_eq!(event.payload["cardId"], card["cardId"]);
    }
    assert_eq!(
        state(&session)["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 2
    );
    assert_eq!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

fn token_stealth_conditional_manifest(seed: u32, north_spellbook_count: usize) -> String {
    let mut stealth = minion(1, 2);
    stealth["stealth"] = json!(true);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &stealth,
        &stealth,
        6,
        north_spellbook_count,
        5,
    );
    let mut site_card = adjacent_combo_site(&json!({
        "genesisEnemiesLoseStealth": true,
        "genesisGainManaIfOnlyControlledCopy": 1,
    }));
    site_card["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("adjacent-combo-site".to_owned(), site_card);
    value["cards"]["foot-soldier"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    value["decks"]["north"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    value["decks"]["south"]["atlas"] = json!(vec!["adjacent-combo-site"; 6]);
    finish_manifest(value)
}

fn heal_bottom_conditional_site() -> Value {
    let mut card = site();
    card.as_object_mut().expect("heal-bottom site").extend(
        json!({
            "genesisGainManaIfOnlyControlledCopy": 1,
            "genesisHealNearbyAvatars": 3,
            "genesisMayBottomNextSpell": true,
        })
        .as_object()
        .expect("heal-bottom facts")
        .clone(),
    );
    card
}

fn adjacent_heal_bottom_conditional_manifest(
    seed: u32,
    north_atlas_count: usize,
    north_spellbook_count: usize,
) -> String {
    let mut loss = minion(1, 2);
    loss["genesisLoseControllerLife"] = json!(2);
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &loss,
        &minion(1, 2),
        north_atlas_count,
        north_spellbook_count,
        5,
    );
    value["cards"]["setup-site"] = site();
    value["cards"]["heal-bottom-site"] = heal_bottom_conditional_site();
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("south-site");
    let mut north_atlas = vec!["setup-site"; north_atlas_count.saturating_sub(2)];
    north_atlas.extend(["heal-bottom-site", "heal-bottom-site"]);
    value["decks"]["north"]["atlas"] = json!(north_atlas);
    value["decks"]["south"]["atlas"] = json!(vec!["setup-site"; 6]);
    finish_manifest(value)
}

fn immobilize_reorder_conditional_genesis_facts() -> Value {
    json!({
        "genesisGainManaIfOnlyControlledCopy": 1,
        "genesisImmobilizeNearbyUntilNextTurn": true,
        "genesisReorderNextSpells": 3,
    })
}

#[test]
fn rule_catalog_0471_site_genesis_conditional_mana_pay_token_strip_stealth_per_adjacent() {
    let manifest = token_stealth_conditional_manifest(471, 10);
    let mut session =
        Session::new(&manifest).expect("valid conditional token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C3",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "site-played",
            "minion-summoned",
            "stealth-lost",
            "spell-drawn"
        ]
    );
    assert_eq!(
        receipt.events[1].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0472_site_genesis_conditional_mana_decline_token_strip_stealth_per_adjacent() {
    let manifest = token_stealth_conditional_manifest(472, 10);
    let mut session =
        Session::new(&manifest).expect("valid conditional token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let south_id = setup_adjacent_stealth_minions(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "stealth-lost", "spell-drawn"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": receipt.events[0].payload["instanceId"],
        })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0473_site_genesis_conditional_mana_pay_token_on_first_controlled_copy() {
    let manifest = token_stealth_conditional_manifest(473, 6);
    let mut session =
        Session::new(&manifest).expect("valid conditional token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let origin_state_version = origin["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_instance_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "mana-gained", "minion-summoned"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(
        receipt.events[2].payload["instanceId"],
        json!(expected_token_id)
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0474_site_genesis_conditional_mana_decline_token_on_first_controlled_copy() {
    let manifest = token_stealth_conditional_manifest(474, 6);
    let mut session =
        Session::new(&manifest).expect("valid conditional token stealth Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let origin = state(&session);
    let source_instance_id = origin["players"]["north"]["hand"]["atlas"][0]["instanceId"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["cardInstanceId"] == source_instance_id
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(event_types(&receipt), ["site-played", "mana-gained"]);
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    assert_eq!(state(&session)["realm"]["units"], json!([]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0475_site_genesis_conditional_mana_heal_nearby_then_bottom_next_spell() {
    let manifest = adjacent_heal_bottom_conditional_manifest(475, 6, 10);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional heal-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_heal_play(&mut session);
    let mana_before = state(&session)["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardId"] == "heal-bottom-site"
    });

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "avatar-healed", "mana-gained"]
    );
    assert_eq!(play_receipt.events[1].payload["amount"], 2);
    assert_eq!(play_receipt.events[2].payload["amount"], 1);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["north"]["mana"], mana_before + 2);
    assert_eq!(state(&session)["phase"], "genesis");

    let mut bottomed = session;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        state(&bottomed)["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&bottomed);
}

#[test]
fn rule_catalog_0476_site_genesis_conditional_mana_heal_nearby_then_keep_next_spell() {
    let manifest = adjacent_heal_bottom_conditional_manifest(476, 6, 10);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mut session =
        Session::new(&manifest).expect("valid adjacent conditional heal-bottom Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    reach_adjacent_heal_play(&mut session);
    let mana_before = state(&session)["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (_, play_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["cardId"] == "heal-bottom-site"
    });

    assert_eq!(
        event_types(&play_receipt),
        ["site-played", "avatar-healed", "mana-gained"]
    );
    assert_eq!(state(&session)["phase"], "genesis");
    assert_eq!(state(&session)["players"]["north"]["mana"], mana_before + 2);

    let mut kept = session;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(state(&kept)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0477_site_genesis_conditional_mana_immobilize_nearby_then_reorder_next_spells() {
    let manifest =
        private_site_genesis_manifest(477, &immobilize_reorder_conditional_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(play_receipt.events[1].payload["amount"], 1);
    assert_eq!(state(&played)["players"]["north"]["mana"], mana_before + 2);
    assert_eq!(
        state(&played)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        }])
    );
    assert_eq!(state(&played)["phase"], "genesis");

    let mut reordered = played;
    let (_, reordered_receipt) = accept_where(&mut reordered, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    assert_eq!(event_types(&reordered_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&reordered)["players"]["north"]["spellbook"][0],
        before_spellbook[2]
    );
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&reordered_receipt.events)
            .expect("reordered events")
            .contains(hidden_top_id)
    );
    assert_exact_replay(&reordered);
}

#[test]
fn rule_catalog_0478_site_genesis_conditional_mana_immobilize_nearby_then_keep_spell_order() {
    let manifest =
        private_site_genesis_manifest(478, &immobilize_reorder_conditional_genesis_facts(), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);

    assert_eq!(event_types(&play_receipt), ["site-played", "mana-gained"]);
    assert_eq!(state(&played)["players"]["north"]["mana"], mana_before + 2);
    assert_eq!(
        state(&played)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        }])
    );

    let mut kept = played;
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    assert_eq!(event_types(&kept_receipt), ["spells-reordered"]);
    assert_eq!(
        state(&kept)["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(state(&kept)["players"]["north"]["mana"], mana_before + 2);
    assert_exact_replay(&kept);
}

#[test]
fn rule_catalog_0850_seasonal_river_genesis_privately_keeps_or_bottoms_next_spell() {
    let manifest =
        private_site_genesis_manifest(160, &json!({ "genesisMayBottomNextSpell": true }), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let top = before_spellbook[0].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpell"],
        json!({
            "seat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        })
    );

    let choices = played.legal_actions().expect("private Genesis choices");
    assert_checkpoint_round_trip(&played);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["kind"], "resolve-genesis-spell");
    assert_eq!(choices[0].descriptor["choice"], "bottom-next");
    assert_eq!(choices[1].descriptor["choice"], "keep-next");
    assert_ne!(choices[0].action_id, choices[1].action_id);
    for choice in &choices {
        let descriptor = serde_json::to_string(&choice.descriptor).expect("choice descriptor");
        assert!(
            choice
                .label
                .contains(top["cardId"].as_str().expect("top card ID"))
        );
        assert!(!descriptor.contains(top["cardId"].as_str().expect("top card ID")));
        assert!(!descriptor.contains(top["instanceId"].as_str().expect("top instance ID")));
    }

    let mut kept = played.clone();
    let (_, kept_receipt) = accept_where(&mut kept, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    let mut bottomed = played;
    let (_, bottomed_receipt) = accept_where(&mut bottomed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "bottom-next"
    });
    let kept_state = state(&kept);
    let bottomed_state = state(&bottomed);
    let source_instance_id = play["cardInstanceId"].clone();

    assert_eq!(kept_state["phase"], "main");
    assert_eq!(bottomed_state["phase"], "main");
    assert_eq!(kept_state["stateVersion"], before_version + 2);
    assert_eq!(bottomed_state["stateVersion"], before_version + 2);
    assert_eq!(kept_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(bottomed_state["pendingGenesisSpell"], Value::Null);
    assert_eq!(
        kept_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    let mut rotated = before_spellbook
        .as_array()
        .expect("Spellbook cards")
        .clone();
    let first = rotated.remove(0);
    rotated.push(first);
    assert_eq!(
        bottomed_state["players"]["north"]["spellbook"],
        Value::Array(rotated)
    );
    assert_eq!(event_types(&kept_receipt), ["spell-kept"]);
    assert_eq!(event_types(&bottomed_receipt), ["spell-bottomed"]);
    assert_eq!(
        kept_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert_eq!(
        bottomed_receipt.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": source_instance_id })
    );
    assert!(kept_receipt.random_draws.is_empty());
    assert!(bottomed_receipt.random_draws.is_empty());
    let hidden_top_id = top["instanceId"].as_str().expect("top instance ID");
    assert!(
        !serde_json::to_string(&kept_receipt.events)
            .expect("kept events")
            .contains(hidden_top_id)
    );
    assert!(
        !serde_json::to_string(&bottomed_receipt.events)
            .expect("bottomed events")
            .contains(hidden_top_id)
    );
    assert_eq!(
        replay_game(&kept).observe(Seat::South),
        replay_game(&bottomed).observe(Seat::South)
    );
    assert_exact_replay(&kept);
    assert_exact_replay(&bottomed);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers prefix permutations, privacy, and short Spellbooks"
)]
fn rule_catalog_0858_observatory_genesis_privately_reorders_next_three_spells() {
    let manifest = private_site_genesis_manifest(161, &json!({ "genesisReorderNextSpells": 3 }), 6);
    let before = state(&opening_checkpoint(&manifest));
    let before_spellbook = before["players"]["north"]["spellbook"].clone();
    let (played, play, play_receipt) = play_private_genesis_site(&manifest);
    let played_state = state(&played);
    let before_version = before["stateVersion"].as_u64().expect("state version");

    assert_eq!(event_types(&play_receipt), ["site-played"]);
    assert!(play_receipt.random_draws.is_empty());
    assert_eq!(played_state["phase"], "genesis");
    assert_eq!(played_state["stateVersion"], before_version + 1);
    assert_eq!(
        played_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        played_state["pendingGenesisSpellOrder"],
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        })
    );

    let choices = played.legal_actions().expect("private order choices");
    assert_checkpoint_round_trip(&played);
    let orders: Vec<_> = choices
        .iter()
        .map(|choice| choice.descriptor["order"].clone())
        .collect();
    assert_eq!(
        orders,
        [
            json!([0, 1, 2]),
            json!([0, 2, 1]),
            json!([1, 0, 2]),
            json!([1, 2, 0]),
            json!([2, 0, 1]),
            json!([2, 1, 0]),
        ]
    );
    let top_instance_id = before_spellbook[0]["instanceId"]
        .as_str()
        .expect("top instance ID");
    assert!(choices.iter().all(|choice| {
        choice.descriptor["kind"] == "resolve-genesis-spell-order"
            && choice
                .label
                .contains(before_spellbook[0]["cardId"].as_str().expect("top card ID"))
            && !serde_json::to_string(&choice.descriptor)
                .expect("order descriptor")
                .contains(top_instance_id)
    }));

    let mut identity = played.clone();
    let (_, identity_receipt) = accept_where(&mut identity, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    let mut reversed = played;
    let (_, reversed_receipt) = accept_where(&mut reversed, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([2, 1, 0])
    });
    let identity_state = state(&identity);
    let reversed_state = state(&reversed);
    let before_cards = before_spellbook.as_array().expect("Spellbook cards");

    assert_eq!(
        identity_state["players"]["north"]["spellbook"],
        before_spellbook
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][0],
        before_cards[2]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][1],
        before_cards[1]
    );
    assert_eq!(
        reversed_state["players"]["north"]["spellbook"][2],
        before_cards[0]
    );
    assert_eq!(identity_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(reversed_state["pendingGenesisSpellOrder"], Value::Null);
    assert_eq!(identity_state["stateVersion"], before_version + 2);
    assert_eq!(reversed_state["stateVersion"], before_version + 2);
    assert_eq!(event_types(&identity_receipt), ["spells-reordered"]);
    assert_eq!(event_types(&reversed_receipt), ["spells-reordered"]);
    assert_eq!(
        reversed_receipt.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": play["cardInstanceId"],
        })
    );
    assert!(identity_receipt.random_draws.is_empty());
    assert!(reversed_receipt.random_draws.is_empty());
    assert!(
        !serde_json::to_string(&reversed_receipt.events)
            .expect("reordered events")
            .contains(top_instance_id)
    );
    assert_eq!(
        replay_game(&identity).observe(Seat::South),
        replay_game(&reversed).observe(Seat::South)
    );
    assert_exact_replay(&identity);
    assert_exact_replay(&reversed);

    for (spellbook_count, seed, expected_orders) in [(4, 165, 1), (5, 166, 2)] {
        let short = private_site_genesis_manifest(
            seed,
            &json!({ "genesisReorderNextSpells": 3 }),
            spellbook_count,
        );
        let (short, _, _) = play_private_genesis_site(&short);
        assert_eq!(
            short.legal_actions().expect("short order choices").len(),
            expected_orders
        );
    }
}

#[test]
fn rule_catalog_0851_private_spell_genesis_skips_empty_spellbook() {
    for (facts, pending_field) in [
        (
            json!({ "genesisMayBottomNextSpell": true }),
            "pendingGenesisSpell",
        ),
        (
            json!({ "genesisReorderNextSpells": 3 }),
            "pendingGenesisSpellOrder",
        ),
    ] {
        let manifest = private_site_genesis_manifest(169, &facts, 3);
        let (session, _, receipt) = play_private_genesis_site(&manifest);
        let after = state(&session);
        assert_eq!(event_types(&receipt), ["site-played"]);
        assert_eq!(after["phase"], "main");
        assert!(
            !after
                .as_object()
                .expect("authoritative state")
                .contains_key(pending_field)
        );
        assert_exact_replay(&session);
    }
}

fn geomancer_cards(seed: u32, north_atlas: &[&str], site_facts: &Value) -> Value {
    let mut geomancer = avatar(false, 20);
    geomancer["earthSitePlayCreatesAdjacentRubble"] = json!(true);
    geomancer["replaceAdjacentRubbleWithTopAtlasSite"] = json!(true);
    let mut value = manifest_value(seed, &geomancer, &minion(1, 1), &minion(1, 1), 4, 8, 4);
    let cards = value["cards"].as_object_mut().expect("card definitions");
    cards.remove("north-site");
    cards.remove("south-site");
    let mut earth_site = site();
    earth_site
        .as_object_mut()
        .expect("earth site facts")
        .extend(site_facts.as_object().expect("Genesis facts").clone());
    for card_id in north_atlas {
        cards.insert((*card_id).to_owned(), earth_site.clone());
    }
    for card_id in [
        "south-site-1",
        "south-site-2",
        "south-site-3",
        "south-site-4",
    ] {
        cards.insert(
            card_id.to_owned(),
            json!({ "cardType": "site", "elements": [] }),
        );
    }
    if earth_site.get("genesisPayOneManaToSummonToken").is_some() {
        cards.insert(
            "foot-soldier".to_owned(),
            json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "token": true,
            }),
        );
    }
    value["decks"]["north"]["atlas"] = json!(north_atlas);
    value["decks"]["south"]["atlas"] = json!([
        "south-site-1",
        "south-site-2",
        "south-site-3",
        "south-site-4",
    ]);
    value
}

fn geomancer_manifest(seed: u32, north_atlas: &[&str], site_facts: &Value) -> String {
    finish_manifest(geomancer_cards(seed, north_atlas, site_facts))
}

fn geomancer_through_south_turn(mut session: Session) -> Session {
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn play_geomancer_c4_rubble_c3(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["createRubbleAt"] == "C3"
    });
    receipt
}

fn replace_rubble_at_c3(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C3"
    });
    receipt
}

#[test]
fn rule_catalog_0852_hidden_spell_genesis_resumes_after_private_rubble_replacement() {
    for (seed, facts, action_kind, pending_field) in [
        (
            170,
            json!({ "genesisMayBottomNextSpell": true }),
            "resolve-genesis-spell",
            "pendingGenesisSpell",
        ),
        (
            171,
            json!({ "genesisReorderNextSpells": 3 }),
            "resolve-genesis-spell-order",
            "pendingGenesisSpellOrder",
        ),
    ] {
        let atlas = [
            "private-site-1",
            "private-site-2",
            "private-site-3",
            "private-site-4",
        ];
        let manifest = geomancer_manifest(seed, &atlas, &facts);
        let mut session = opening_checkpoint(&manifest);
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cell"] == "C4"
                && descriptor["createRubbleAt"] == "C3"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == action_kind);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });

        let (_, replacement) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "replace-rubble-with-top-atlas-site"
                && descriptor["targetCell"] == "C3"
        });
        assert_eq!(
            event_types(&replacement),
            ["rubble-replaced", "site-played"]
        );
        assert_eq!(state(&session)["phase"], "genesis");
        assert!(state(&session)[pending_field].is_object());
        assert!(
            session
                .legal_actions()
                .expect("replacement Genesis actions")
                .iter()
                .all(|action| action.descriptor["kind"] == action_kind)
        );
        accept_where(&mut session, |descriptor| descriptor["kind"] == action_kind);
        assert_eq!(state(&session)[pending_field], Value::Null);
        assert_exact_replay(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers Geomancer creation, private replacement, and deferred Genesis"
)]
fn rule_catalog_0859_geomancer_creates_rubble_and_privately_replaces_with_top_atlas_site() {
    let ready = |manifest: &str| {
        let mut session = Session::new(manifest).expect("valid Geomancer scenario");
        keep(&mut session);
        keep(&mut session);
        let origin = state(&session);
        let avatar_id = origin["players"]["north"]["avatar"]["card"]["instanceId"].clone();
        let expected_rubble_id = identity_hash(&json!({
            "cell": "C3",
            "kind": "rubble",
            "sourceInstanceId": avatar_id,
            "stateVersion": origin["stateVersion"],
        }))
        .expect("deterministic Rubble identity");
        let (_, first) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cell"] == "C4"
                && descriptor["createRubbleAt"] == "C3"
                && descriptor["genesisTokenChoice"] == "decline"
        });
        assert_eq!(event_types(&first), ["site-played", "rubble-created"]);
        assert_eq!(
            first.events[1].payload,
            json!({
                "cell": "C3",
                "instanceId": expected_rubble_id,
                "sourceInstanceId": avatar_id,
            })
        );
        assert_eq!(state(&session)["realm"]["sites"]["C3"]["rubble"], true);
        assert!(first.random_draws.is_empty());

        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        session
    };

    let north_atlas = [
        "rustic-village-1",
        "rustic-village-2",
        "rustic-village-3",
        "rustic-village-4",
    ];
    let token_facts = json!({ "genesisPayOneManaToSummonToken": "foot-soldier" });
    let manifest = geomancer_manifest(104, &north_atlas, &token_facts);
    let mut session = ready(&manifest);
    let before = state(&session);
    let top = before["players"]["north"]["atlas"][0].clone();
    let replacement = session
        .legal_actions()
        .expect("replacement actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "replace-rubble-with-top-atlas-site"
                && action.descriptor["targetCell"] == "C3"
        })
        .expect("engine-issued private replacement");
    assert_eq!(
        replacement.label,
        "Replace Rubble at C3 with the top site of your Atlas"
    );
    let replacement_json = serde_json::to_string(&replacement).expect("replacement JSON");
    assert!(!replacement_json.contains(top["cardId"].as_str().expect("top card ID")));
    assert!(!replacement_json.contains(top["instanceId"].as_str().expect("top instance ID")));

    let mut covered = session.clone();
    let (_, covered_receipt) = accept_where(&mut covered, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor["createRubbleAt"] == "B4"
            && descriptor["genesisTokenChoice"] == "decline"
    });
    assert_eq!(
        event_types(&covered_receipt),
        ["rubble-replaced", "site-played", "rubble-created"]
    );
    assert_eq!(
        state(&covered)["realm"]["sites"]["C3"]["rubble"],
        Value::Null
    );
    assert_eq!(state(&covered)["realm"]["sites"]["B4"]["rubble"], true);
    assert_exact_replay(&covered);

    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"].clone();
    let mut moved = session.clone();
    let (_, movement) = accept_where(&mut moved, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id
            && descriptor["to"]["cell"] == "C3"
    });
    assert_eq!(event_types(&movement), ["move-and-attack-activated"]);
    assert_eq!(
        state(&moved)["players"]["north"]["avatar"]["location"],
        "C3"
    );
    accept_where(&mut moved, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_exact_replay(&moved);

    let rotated = [
        "rustic-village-2",
        "rustic-village-3",
        "rustic-village-4",
        "rustic-village-1",
    ];
    let hidden_alternative = ready(&geomancer_manifest(104, &rotated, &token_facts));
    let alternative_top = state(&hidden_alternative)["players"]["north"]["atlas"][0].clone();
    assert_ne!(alternative_top["cardId"], top["cardId"]);
    let alternative_replacement = hidden_alternative
        .legal_actions()
        .expect("alternative replacement actions")
        .into_iter()
        .find(|action| action.descriptor["kind"] == "replace-rubble-with-top-atlas-site")
        .expect("alternative private replacement");
    assert_eq!(alternative_replacement.descriptor, replacement.descriptor);
    assert_eq!(alternative_replacement.action_id, replacement.action_id);

    let StepResult::Accepted(replaced) = session
        .step(ActionRequest {
            action_id: replacement.action_id.to_string(),
            seat: replacement.seat,
            state_version: replacement.state_version,
        })
        .expect("authoritative Rubble replacement")
    else {
        panic!("engine-issued Rubble replacement must be accepted");
    };
    assert_eq!(event_types(&replaced), ["rubble-replaced", "site-played"]);
    assert!(replaced.random_draws.is_empty());
    let replaced_state = state(&session);
    assert_eq!(replaced_state["phase"], "genesis");
    assert_eq!(replaced_state["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(replaced_state["players"]["north"]["atlas"], json!([]));
    assert_eq!(
        replaced_state["players"]["north"]["hand"]["atlas"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        replaced_state["realm"]["sites"]["C3"]["instanceId"],
        top["instanceId"]
    );
    assert_eq!(
        replaced_state["realm"]["sites"]
            .as_object()
            .expect("realm sites")
            .values()
            .filter(|site| site["rubble"] == true)
            .count(),
        0
    );
    assert_eq!(
        replaced_state["pendingGenesisToken"]["sourceInstanceId"],
        top["instanceId"]
    );

    let choices = session.legal_actions().expect("deferred Genesis choices");
    assert_eq!(choices.len(), 2);
    assert!(
        choices
            .iter()
            .all(|action| action.descriptor["kind"] == "resolve-genesis-token")
    );
    let expected_token_id = identity_hash(&json!({
        "cardId": "foot-soldier",
        "cell": "C3",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": top["instanceId"],
        "stateVersion": replaced_state["stateVersion"],
    }))
    .expect("deterministic deferred token identity");
    let (_, paid) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-token" && descriptor["choice"] == "pay-one-mana"
    });
    assert_eq!(event_types(&paid), ["minion-summoned"]);
    assert!(paid.random_draws.is_empty());
    let final_state = state(&session);
    assert_eq!(final_state["phase"], "main");
    assert_eq!(final_state["pendingGenesisToken"], Value::Null);
    assert_eq!(final_state["players"]["north"]["mana"], 1);
    assert_eq!(final_state["realm"]["units"][0]["cardId"], "foot-soldier");
    assert_eq!(
        final_state["realm"]["units"][0]["instanceId"],
        json!(expected_token_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0719_geomancer_admits_site_genesis_on_owners_atlas() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0719_geomancer_admits_site_genesis_on_owners_atlas();
}

#[test]
fn rule_catalog_0162_rubble_replacement_resumes_adjacent_same_card_spell_draws() {
    let atlas = ["leyline", "leyline", "leyline", "leyline"];
    let manifest = geomancer_manifest(
        180,
        &atlas,
        &json!({ "genesisDrawSpellPerAdjacentSameCard": true }),
    );
    let mut session = opening_checkpoint(&manifest);
    let first = play_geomancer_c4_rubble_c3(&mut session);
    assert_eq!(event_types(&first), ["site-played", "rubble-created"]);
    session = geomancer_through_south_turn(session);
    let replacement = replace_rubble_at_c3(&mut session);
    assert_eq!(
        event_types(&replacement),
        ["rubble-replaced", "site-played", "spell-drawn"]
    );
    assert_eq!(
        replacement.events[2].payload,
        json!({
            "seat": "north",
            "sourceInstanceId": replacement.events[1].payload["instanceId"],
        })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0853_rubble_replacement_discards_top_spells_like_play_site() {
    let atlas = ["cemetery-1", "cemetery-2", "cemetery-3", "cemetery-4"];
    let manifest = geomancer_manifest(181, &atlas, &json!({ "genesisDiscardTopSpells": 2 }));
    let mut session = opening_checkpoint(&manifest);
    let first = play_geomancer_c4_rubble_c3(&mut session);
    assert_eq!(
        event_types(&first),
        [
            "site-played",
            "spell-discarded",
            "spell-discarded",
            "rubble-created"
        ]
    );
    session = geomancer_through_south_turn(session);
    let replacement = replace_rubble_at_c3(&mut session);
    assert_eq!(
        event_types(&replacement),
        [
            "rubble-replaced",
            "site-played",
            "spell-discarded",
            "spell-discarded"
        ]
    );
    assert_eq!(
        replacement.events[2].payload["sourceInstanceId"],
        replacement.events[1].payload["instanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0854_rubble_replacement_immobilizes_nearby_after_first_area_expires() {
    let atlas = ["trap-1", "trap-2", "trap-3", "trap-4"];
    let manifest = geomancer_manifest(
        182,
        &atlas,
        &json!({ "genesisImmobilizeNearbyUntilNextTurn": true }),
    );
    let mut session = opening_checkpoint(&manifest);
    let first = play_geomancer_c4_rubble_c3(&mut session);
    assert_eq!(event_types(&first), ["site-played", "rubble-created"]);
    assert_eq!(
        state(&session)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": first.events[0].payload["instanceId"],
        }])
    );
    session = geomancer_through_south_turn(session);
    assert_eq!(state(&session)["realm"]["immobileAreas"], Value::Null);
    let replacement = replace_rubble_at_c3(&mut session);
    assert_eq!(
        event_types(&replacement),
        ["rubble-replaced", "site-played"]
    );
    assert_eq!(
        state(&session)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C3", "C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": replacement.events[1].payload["instanceId"],
        }])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0855_rubble_replacement_strips_enemy_stealth_like_play_site() {
    let atlas = ["lodge-1", "lodge-2", "lodge-3", "lodge-4"];
    let mut value = geomancer_cards(183, &atlas, &json!({ "genesisEnemiesLoseStealth": true }));
    value["cards"]["north-minion"]["stealth"] = json!(true);
    value["cards"]["south-minion"]["stealth"] = json!(true);
    value["cards"]["south-minion"]["thresholds"]["earth"] = json!(0);
    let manifest = finish_manifest(value);
    let mut session = opening_checkpoint(&manifest);
    play_geomancer_c4_rubble_c3(&mut session);
    let (north_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let north_id = north_summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (south_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
    });
    let south_id = south_summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let replacement = replace_rubble_at_c3(&mut session);
    assert_eq!(
        event_types(&replacement),
        ["rubble-replaced", "site-played", "stealth-lost"]
    );
    assert_eq!(
        replacement.events[2].payload,
        json!({
            "instanceId": south_id,
            "seat": "south",
            "sourceInstanceId": replacement.events[1].payload["instanceId"],
        })
    );
    let after = state(&session);
    let units = after["realm"]["units"].as_array().expect("realm units");
    assert!(
        units
            .iter()
            .any(|unit| unit["instanceId"] == north_id && unit["stealthed"] == true)
    );
    assert!(
        units
            .iter()
            .any(|unit| unit["instanceId"] == south_id && unit["stealthed"] == false)
    );
    assert_exact_replay(&session);
}

fn single_copy_mixed_mana_manifest(seed: u32) -> String {
    let mut value = manifest_value(
        seed,
        &avatar(false, 20),
        &minion(1, 2),
        &minion(1, 2),
        8,
        8,
        8,
    );
    let mut mixed = site();
    mixed["genesisGainMana"] = json!(2);
    mixed["genesisGainManaIfOnlyControlledCopy"] = json!(1);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("mixed-mana-site".to_owned(), mixed);
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .insert("north-filler-site".to_owned(), site());
    value["cards"]
        .as_object_mut()
        .expect("card definitions")
        .remove("north-site");
    value["decks"]["north"]["atlas"] = json!([
        "mixed-mana-site",
        "north-filler-site",
        "mixed-mana-site",
        "north-filler-site",
        "mixed-mana-site",
        "north-filler-site",
        "mixed-mana-site",
        "north-filler-site",
    ]);
    finish_manifest(value)
}

#[test]
fn rule_catalog_0497_site_genesis_mixed_mana_grants_unconditional_and_conditional_on_first_copy() {
    let manifest = single_copy_mixed_mana_manifest(497);
    let mut session = Session::new(&manifest).expect("valid mixed-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let before = state(&session);
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let (_, first_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "mixed-mana-site"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&first_play), ["site-played", "mana-gained"]);
    assert_eq!(
        first_play.events[1].payload,
        json!({
            "amount": 3,
            "seat": "north",
            "sourceInstanceId": first_play.events[0].payload["instanceId"],
        })
    );
    assert_eq!(
        state(&session)["players"]["north"]["mana"]
            .as_u64()
            .expect("north mana"),
        mana_before + 4
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0498_site_genesis_mixed_mana_grants_only_unconditional_on_later_copy() {
    let manifest = single_copy_mixed_mana_manifest(498);
    let mut session = Session::new(&manifest).expect("valid mixed-mana Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    let (_, first_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "mixed-mana-site"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(event_types(&first_play), ["site-played", "mana-gained"]);
    assert_eq!(first_play.events[1].payload["amount"], json!(3));
    assert_eq!(state(&session)["players"]["north"]["mana"], 4);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, filler_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-filler-site"
            && descriptor["cell"] == "C3"
    });
    assert_eq!(event_types(&filler_play), ["site-played"]);
    assert_eq!(state(&session)["players"]["north"]["mana"], 2);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, second_play) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "mixed-mana-site"
            && descriptor["cell"] == "B3"
    });
    assert_eq!(event_types(&second_play), ["site-played", "mana-gained"]);
    assert_eq!(
        second_play.events[1].payload,
        json!({
            "amount": 2,
            "seat": "north",
            "sourceInstanceId": second_play.events[0].payload["instanceId"],
        })
    );
    assert_eq!(state(&session)["players"]["north"]["mana"], 5);
    assert_exact_replay(&session);
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_draw_site_minion() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 1,
        "genesisDrawSite": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn deathrite_genesis_draw_manifest(seed: u32) -> String {
    let fixture = "genesis-draw-site-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(false, 20),
            "north-genesis": genesis_draw_site_minion(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(false, 20),
            "south-deathrite": deathrite_plain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-genesis",
                    "north-rain",
                    "north-rain",
                    "north-genesis",
                    "north-rain",
                    "north-genesis",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_genesis_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-genesis", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn genesis_summon_offered(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-genesis"
        })
    })
}

struct PendingDeathriteGenesisDrawSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_genesis_draw_site_in_hand(
    encoded: &str,
) -> Option<PendingDeathriteGenesisDrawSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_genesis_and_rain(&state(&session)) {
        return None;
    }
    if !genesis_summon_offered(&session) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGenesisDrawSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_genesis_draw_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_genesis_draw_manifest)
        .find(|candidate| try_pending_deathrite_with_genesis_draw_site_in_hand(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with genesis draw-site minion in hand",
        )
}

#[test]
fn rule_catalog_1110_genesis_draw_site_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_genesis_draw_seed_with(1110);
    let mut setup = try_pending_deathrite_with_genesis_draw_site_in_hand(&encoded)
        .expect("complete genesis-draw-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "end-turn"
                    && action.descriptor["kind"] != "summon-minion"
            })
    );
    assert!(!genesis_summon_offered(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(genesis_summon_offered(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-genesis"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "site-drawn"]);
    assert_exact_replay(session);
}

fn genesis_draw_spell_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDrawSpells": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_draw_spell_deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_draw_spell_rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn try_accept_genesis_draw_spell_where(
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

fn deathrite_genesis_draw_spell_manifest(seed: u32) -> String {
    let fixture = "genesis-draw-spell-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(false, 20),
            "north-draw-spell": genesis_draw_spell_minion(),
            "north-rain": genesis_draw_spell_rain(),
            "north-site": site(),
            "south-avatar": avatar(false, 20),
            "south-deathrite": genesis_draw_spell_deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-draw-spell",
                    "north-rain",
                    "north-rain",
                    "north-draw-spell",
                    "north-rain",
                    "north-draw-spell",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_genesis_draw_spell_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-draw-spell", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn genesis_draw_spell_summon_offered(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-draw-spell"
        })
    })
}

struct PendingDeathriteGenesisDrawSpellSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_genesis_draw_spell_in_hand(
    encoded: &str,
) -> Option<PendingDeathriteGenesisDrawSpellSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_genesis_draw_spell_and_rain(&state(&session)) {
        return None;
    }
    if !genesis_draw_spell_summon_offered(&session) {
        return None;
    }
    try_accept_genesis_draw_spell_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGenesisDrawSpellSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_genesis_draw_spell_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_genesis_draw_spell_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_genesis_draw_spell_in_hand(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with genesis draw-spell minion in hand",
        )
}

#[test]
fn rule_catalog_1112_genesis_draw_spell_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_genesis_draw_spell_seed_with(1112);
    let mut setup = try_pending_deathrite_with_genesis_draw_spell_in_hand(&encoded)
        .expect("complete genesis-draw-spell Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "end-turn"
                    && action.descriptor["kind"] != "summon-minion"
            })
    );
    assert!(!genesis_draw_spell_summon_offered(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(genesis_draw_spell_summon_offered(session));

    let before = state(session);
    let drawn_id = before["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-draw-spell"
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "spell-drawn"]);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn_id)
    );
    assert_exact_replay(session);
}

fn burrowing_deathrite() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_genesis_spell_choice_manifest(seed: u32) -> String {
    let fixture = "genesis-spell-choice-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "deathrite-1": burrowing_deathrite(),
            "deathrite-2": burrowing_deathrite(),
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
                "replaceAdjacentRubbleWithTopAtlasSite": true,
            },
            "north-minion": minion(1, 1),
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "sacrificeToDestroyNearbySite": true,
            },
            "south-avatar": avatar(false, 20),
            "south-minion": minion(1, 1),
            "south-site": site(),
            "water-site": {
                "cardType": "site",
                "elements": ["water"],
                "genesisMayBottomNextSpell": true,
            },
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-site",
                    "north-site",
                    "north-site",
                    "north-site",
                    "water-site",
                    "north-site",
                    "north-site",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "deathrite-1",
                    "deathrite-2",
                    "deathrite-1",
                    "deathrite-2",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                ],
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
    }))
}

fn try_end_turn_draw_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn resolve_genesis_spell_offered(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "resolve-genesis-spell"
                && matches!(
                    action.descriptor["choice"].as_str(),
                    Some("keep-next" | "bottom-next")
                )
        })
    })
}

struct PendingDeathriteGenesisSpellChoiceSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_genesis_spell_choice(
    encoded: &str,
) -> Option<PendingDeathriteGenesisSpellChoiceSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "deathrite-1"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "deathrite-2"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-site-destruction" && descriptor["targetCell"] == "C3"
    })?;
    let before = state(&session);
    if before["players"]["north"]["atlas"].get(0)?["cardId"] != "water-site" {
        return None;
    }
    if before["players"]["north"]["atlas"].as_array()?.len() < 3 {
        return None;
    }
    if before["players"]["north"]["spellbook"]
        .as_array()?
        .is_empty()
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C3"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if resolve_genesis_spell_offered(&session) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGenesisSpellChoiceSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_genesis_spell_choice_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_genesis_spell_choice_manifest)
        .find(|candidate| try_pending_deathrite_during_genesis_spell_choice(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites before Genesis spell-draw choices")
}

#[test]
fn rule_catalog_1164_resolve_genesis_spell_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_genesis_spell_choice_seed_with(1164);
    let mut setup = try_pending_deathrite_during_genesis_spell_choice(&encoded)
        .expect("complete Genesis spell-choice Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "north");
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["kind"],
        "site-genesis"
    );
    assert!(paused["pendingGenesisSpell"].is_null());
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-genesis-spell")
    );
    assert!(!resolve_genesis_spell_offered(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "genesis");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(resumed["pendingGenesisSpell"].is_object());
    let choices = session
        .legal_actions()
        .expect("resumed Genesis spell-draw choices");
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].descriptor["kind"], "resolve-genesis-spell");
    assert_eq!(choices[0].descriptor["choice"], "bottom-next");
    assert_eq!(choices[1].descriptor["choice"], "keep-next");
    assert!(resolve_genesis_spell_offered(session));

    let before_spellbook = resumed["players"]["north"]["spellbook"].clone();
    let (_, kept) = accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell" && descriptor["choice"] == "keep-next"
    });
    assert_eq!(event_types(&kept), ["spell-kept"]);
    let after = state(session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingGenesisSpell"].is_null());
    assert_eq!(after["players"]["north"]["spellbook"], before_spellbook);
    assert_exact_replay(session);
}
fn deathrite_genesis_spell_order_manifest(seed: u32) -> String {
    let fixture = "genesis-spell-order-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "deathrite-1": burrowing_deathrite(),
            "deathrite-2": burrowing_deathrite(),
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
                "replaceAdjacentRubbleWithTopAtlasSite": true,
            },
            "north-minion": minion(1, 1),
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "sacrificeToDestroyNearbySite": true,
            },
            "south-avatar": avatar(false, 20),
            "south-minion": minion(1, 1),
            "south-site": site(),
            "water-site": {
                "cardType": "site",
                "elements": ["water"],
                "genesisReorderNextSpells": 3,
            },
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-site",
                    "north-site",
                    "north-site",
                    "north-site",
                    "water-site",
                    "north-site",
                    "north-site",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "deathrite-1",
                    "deathrite-2",
                    "deathrite-1",
                    "deathrite-2",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                    "north-minion",
                ],
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
    }))
}

fn resolve_genesis_spell_order_offered(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == "resolve-genesis-spell-order")
    })
}

struct PendingDeathriteGenesisSpellOrderSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_genesis_spell_order(
    encoded: &str,
) -> Option<PendingDeathriteGenesisSpellOrderSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "deathrite-1"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "deathrite-2"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_end_turn_draw_spellbook(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-site-destruction" && descriptor["targetCell"] == "C3"
    })?;
    let before = state(&session);
    if before["players"]["north"]["atlas"].get(0)?["cardId"] != "water-site" {
        return None;
    }
    if before["players"]["north"]["atlas"].as_array()?.len() < 3 {
        return None;
    }
    if before["players"]["north"]["spellbook"].as_array()?.len() < 3 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C3"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if resolve_genesis_spell_order_offered(&session) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGenesisSpellOrderSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_genesis_spell_order_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_genesis_spell_order_manifest)
        .find(|candidate| try_pending_deathrite_during_genesis_spell_order(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites before Genesis spell-order choices")
}

#[test]
fn rule_catalog_1165_resolve_genesis_spell_order_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_genesis_spell_order_seed_with(1165);
    let mut setup = try_pending_deathrite_during_genesis_spell_order(&encoded)
        .expect("complete Genesis spell-order Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "north");
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["kind"],
        "site-genesis"
    );
    assert!(paused["pendingGenesisSpellOrder"].is_null());
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-genesis-spell-order")
    );
    assert!(!resolve_genesis_spell_order_offered(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "genesis");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(resumed["pendingGenesisSpellOrder"]["count"], 3);
    assert_eq!(resumed["pendingGenesisSpellOrder"]["seat"], "north");
    let choices = session
        .legal_actions()
        .expect("resumed Genesis spell-order choices");
    assert_eq!(choices.len(), 6);
    assert!(
        choices
            .iter()
            .all(|choice| choice.descriptor["kind"] == "resolve-genesis-spell-order")
    );
    assert!(resolve_genesis_spell_order_offered(session));

    let before_spellbook = resumed["players"]["north"]["spellbook"].clone();
    let source_instance_id = resumed["pendingGenesisSpellOrder"]["sourceInstanceId"].clone();
    let (_, kept) = accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-genesis-spell-order"
            && descriptor["order"] == json!([0, 1, 2])
    });
    assert_eq!(event_types(&kept), ["spells-reordered"]);
    assert_eq!(
        kept.events[0].payload,
        json!({
            "count": 3,
            "seat": "north",
            "sourceInstanceId": source_instance_id,
        })
    );
    let after = state(session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingGenesisSpellOrder"].is_null());
    assert_eq!(after["players"]["north"]["spellbook"], before_spellbook);
    assert_exact_replay(session);
}
