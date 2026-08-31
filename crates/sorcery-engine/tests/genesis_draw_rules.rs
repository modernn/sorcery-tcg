use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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
    let mut session = Session::new(manifest).expect("valid Genesis scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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
fn avatar_draw_spell_should_pay_tap_and_keep_identity_private() {
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
fn genesis_draw_site_should_keep_identity_private_and_deck_out_after_summon() {
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
fn genesis_draw_spell_should_keep_identity_private_and_deck_out_after_summon() {
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
fn numeric_genesis_spell_draws_should_preserve_top_order() {
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
fn numeric_genesis_spell_draws_should_exhaust_before_deck_out() {
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
fn genesis_life_loss_should_reach_but_not_cross_deaths_door() {
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
fn genesis_heal_should_cap_at_max_and_not_heal_deaths_door() {
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
fn undamaged_zero_defense_genesis_minion_should_survive_until_positive_damage() {
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
fn site_genesis_mana_should_pay_summon_and_expire_to_site_count() {
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
fn site_genesis_heal_should_target_both_nearby_avatars_in_seat_order_and_cap() {
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
fn first_controlled_copy_site_genesis_mana_should_key_by_card_id() {
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
fn site_genesis_should_remove_only_enemy_stealth() {
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

#[test]
fn adjacent_matching_site_genesis_should_draw_each_then_partially_deck_out() {
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
fn site_genesis_should_publicly_discard_up_to_two_spells_without_deck_out() {
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
fn optional_site_genesis_should_issue_decline_and_paid_token_branches() {
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

fn geomancer_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut geomancer = avatar(false, 20);
    geomancer["earthSitePlayCreatesAdjacentRubble"] = json!(true);
    geomancer["replaceAdjacentRubbleWithTopAtlasSite"] = json!(true);
    let mut value = manifest_value(seed, &geomancer, &minion(1, 1), &minion(1, 1), 4, 4, 4);
    let cards = value["cards"].as_object_mut().expect("card definitions");
    cards.remove("north-site");
    cards.remove("south-site");
    let mut earth_site = site();
    earth_site["genesisPayOneManaToSummonToken"] = json!("foot-soldier");
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
    value["decks"]["north"]["atlas"] = json!(north_atlas);
    value["decks"]["south"]["atlas"] = json!([
        "south-site-1",
        "south-site-2",
        "south-site-3",
        "south-site-4",
    ]);
    finish_manifest(value)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers Geomancer creation, private replacement, and deferred Genesis"
)]
fn geomancer_should_create_rubble_and_privately_replace_it_with_top_atlas_site() {
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
    let manifest = geomancer_manifest(104, &north_atlas);
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
    let hidden_alternative = ready(&geomancer_manifest(104, &rotated));
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
