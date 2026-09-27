//! Shared continuous protection, topology, and simultaneous damage snapshots.
use super::*;
use crate::synthetic::selfplay_manifest_with;

fn fixture(grant: &Value) -> Game {
    let encoded = selfplay_manifest_with(319, |m| {
        let zero = json!({"earth":0,"fire":0,"water":0,"air":0});
        m["cards"] = json!({
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":{"cardType":"site","elements":["earth"]},
            "provider":{"cardType":"minion","attack":2,"defense":2,"manaCost":0,"thresholds":zero,"nearbyDamagePrevention":grant,"otherNearbyAlliesPowerBonus":1},
            "target":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"thresholds":zero}
        });
        m["decks"] = json!({
            "north":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":vec!["provider";3]},
            "south":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":vec!["target";3]}
        });
    });
    let mut game = Game::from_manifest_json(&encoded).unwrap();
    game.position.phase = Phase::Main;
    game.position.players[0].avatar.location = Cell::parse("C3").unwrap();
    game.position.players[1].avatar.location = Cell::parse("C2").unwrap();
    game
}

fn add_minion(
    game: &mut Game,
    provider: bool,
    seat: Seat,
    cell: &str,
    region: Region,
) -> IdentityHash {
    let hand_seat = if provider { Seat::North } else { Seat::South };
    let card = game.position.players[seat_index(hand_seat)]
        .hand_spellbook
        .remove(0);
    let id = card.instance_id.clone();
    game.position.units.push(
        SummonPlacement {
            card,
            controller: seat,
            lance_count: 0,
            location: Cell::parse(cell).unwrap(),
            occupied_cells: None,
            region,
            stealthed: false,
            warded: false,
        }
        .into_unit(),
    );
    id
}

fn source(origin: DamageOrigin) -> UnitDamageSource {
    UnitDamageSource {
        origin,
        current_power: 3,
        lethal: false,
    }
}

fn damage(
    game: &Game,
    kind: UnitKind,
    seat: Seat,
    id: &IdentityHash,
    origin: DamageOrigin,
    amount: u16,
) -> u16 {
    let status = game.unit_damage_status(kind, seat, id).unwrap();
    Game::damage_after_prevention(&[(amount, source(origin))], status.prevention)
        .unwrap()
        .0
}

#[test]
fn nearby_protection_includes_self_and_stacks_but_obeys_controller_region_and_range() {
    let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &provider,
            DamageOrigin::Other,
            3
        ),
        2
    );
    let target = add_minion(&mut game, false, Seat::North, "D2", Region::Surface);
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            3
        ),
        2
    );
    add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            3
        ),
        1
    );
    game.position.units[1].controller = Seat::South;
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::South,
            &target,
            DamageOrigin::Other,
            3
        ),
        3
    );
    game.position.units[1].controller = Seat::North;
    game.position.units[1].region = Region::Underground;
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            3
        ),
        3
    );
    game.position.units[1].region = Region::Surface;
    game.position.units[1].location = Cell::parse("A1").unwrap();
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            3
        ),
        3
    );
}

#[test]
fn source_suppression_removes_grants_but_target_suppression_does_not() {
    let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    let target = add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
    game.position.units[1].temporary_modifiers.grant(
        TemporaryModifierKind::Silence,
        1,
        provider.clone(),
    );
    game.position.units[1].disabled_until_damaged = true;
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            2
        ),
        1
    );
    game.position.units[0]
        .temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, provider);
    assert_eq!(
        damage(
            &game,
            UnitKind::Minion,
            Seat::North,
            &target,
            DamageOrigin::Other,
            2
        ),
        2
    );
    let avatar = &game.position.players[0].avatar;
    assert_eq!(
        game.avatar_current_stats(Seat::North).unwrap(),
        (1, 1),
        "Silence also removes the existing continuous power grant from avatars"
    );
    assert_eq!(
        damage(
            &game,
            UnitKind::Avatar,
            Seat::North,
            &avatar.card.instance_id,
            DamageOrigin::Other,
            2
        ),
        2
    );
}

#[test]
fn magic_protection_applies_to_both_players_and_avatars_without_blocking_other_damage() {
    let mut game = fixture(&json!({"preventsDamageFrom":"magic"}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    let target = add_minion(&mut game, false, Seat::South, "C3", Region::Surface);
    let magic = DamageOrigin::Magic(ElementSet::only(Element::Fire));
    for (seat, id) in [(Seat::North, provider), (Seat::South, target)] {
        assert_eq!(damage(&game, UnitKind::Minion, seat, &id, magic, 3), 0);
        assert_eq!(
            damage(
                &game,
                UnitKind::Minion,
                seat,
                &id,
                DamageOrigin::RangedStrike,
                3
            ),
            3
        );
    }
    for seat in [Seat::North, Seat::South] {
        let id = &game.position.players[seat_index(seat)]
            .avatar
            .card
            .instance_id;
        assert_eq!(damage(&game, UnitKind::Avatar, seat, id, magic, 3), 0);
        assert_eq!(
            damage(&game, UnitKind::Avatar, seat, id, DamageOrigin::Other, 3),
            3
        );
    }
}

#[test]
fn waking_a_provider_does_not_change_protection_halfway_through_simultaneous_damage() {
    for reverse in [false, true] {
        let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
        let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
        game.position.units[0].disabled_until_damaged = true;
        let avatar = game.position.players[0].avatar.card.instance_id.clone();
        let mut targets = vec![
            (provider.clone(), UnitKind::Minion, Seat::North),
            (avatar.clone(), UnitKind::Avatar, Seat::North),
        ];
        if reverse {
            targets.reverse();
        }
        let mut events = Vec::new();
        game.apply_area_damage(
            targets,
            1,
            source(DamageOrigin::Other),
            ("test-allocation", &provider),
            &mut OutcomeLog::Record(&mut events),
        )
        .unwrap();
        assert_eq!(game.position.players[0].avatar.life, 19);
        assert!(!game.position.units[0].disabled_until_damaged);
        assert_eq!(
            damage(
                &game,
                UnitKind::Avatar,
                Seat::North,
                &avatar,
                DamageOrigin::Other,
                1
            ),
            0
        );
    }
}

#[test]
fn dying_provider_protects_the_whole_group_then_stops_protecting_later_damage() {
    let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
    let avatar = game.position.players[0].avatar.card.instance_id.clone();
    let mut events = Vec::new();
    game.damage_each_unit_at_location(
        Location {
            cell: Cell::parse("C3").unwrap(),
            region: Region::Surface,
        },
        3,
        source(DamageOrigin::Other),
        ("test-allocation", &provider),
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert!(
        game.position
            .units
            .iter()
            .all(|u| u.card.instance_id != provider)
    );
    assert_eq!(game.position.units[0].damage, 2);
    assert_eq!(game.position.players[0].avatar.life, 18);
    game.apply_simple_damage(
        UnitKind::Avatar,
        Seat::North,
        &avatar,
        1,
        source(DamageOrigin::Other),
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert_eq!(game.position.players[0].avatar.life, 17);
}

#[test]
fn provider_selectors_compose_with_printed_reduction() {
    for (grant, matching) in [
        (json!({"takesLessDamage":1}), DamageOrigin::Other),
        (
            json!({"preventsDamageFrom":"ranged-strikes"}),
            DamageOrigin::RangedStrike,
        ),
        (
            json!({"preventsDamageFrom":"fire-magic"}),
            DamageOrigin::Magic(ElementSet::only(Element::Fire)),
        ),
        (
            json!({"preventsDamageFromUnitsWithPowerAtLeast":3}),
            DamageOrigin::Other,
        ),
    ] {
        let mut game = fixture(&grant);
        add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
        let id = add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
        let card_id = game.position.units[1].card.card_id;
        let CardFacts::Minion(facts) =
            &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
        else {
            panic!("minion");
        };
        facts.damage_prevention = Some(DamagePrevention::TakesLessDamage(2));
        assert_eq!(
            damage(&game, UnitKind::Minion, Seat::North, &id, matching, 5),
            if grant.get("takesLessDamage").is_some() {
                2
            } else {
                0
            }
        );
        if grant.get("preventsDamageFrom").is_some() {
            assert_eq!(
                damage(
                    &game,
                    UnitKind::Minion,
                    Seat::North,
                    &id,
                    DamageOrigin::Other,
                    5
                ),
                3
            );
        }
    }
}

#[test]
fn malformed_nearby_protection_is_rejected_at_rust_admission() {
    for grant in [
        json!(null),
        json!([]),
        json!({}),
        json!({"ward":true}),
        json!({"takesLessDamage":0}),
        json!({"takesLessDamage":101}),
        json!({"takesLessDamage":1.5}),
        json!({"takesLessDamage":1,"alliedOnly":false}),
        json!({"takesLessDamage":1,"preventsDamageFrom":"magic"}),
        json!({"preventsDamageFrom":"physical"}),
        json!({"takesLessDamage":1,"extra":true}),
    ] {
        let card = json!({"cardType":"minion","attack":1,"defense":1,"manaCost":0,
            "thresholds":{"earth":0,"fire":0,"water":0,"air":0},"nearbyDamagePrevention":grant});
        assert!(crate::facts::parse_card_definition("invalid", &card).is_err());
    }
}

#[test]
fn end_turn_aura_damage_snapshots_protection_before_waking_a_provider() {
    let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    let target = add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
    game.position.units[0].disabled_until_damaged = true;
    let mut events = Vec::new();
    game.damage_each_unit_here_for_wildfire(
        &provider,
        Cell::parse("C3").unwrap(),
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert_eq!(
        game.position
            .units
            .iter()
            .find(|u| u.card.instance_id == target)
            .unwrap()
            .damage,
        3
    );
    assert_eq!(game.position.players[0].avatar.life, 17);
}

fn set_subtypes(game: &mut Game, id: &IdentityHash, names: &[&str]) {
    let card_id = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == *id)
        .unwrap()
        .card
        .card_id;
    let CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
    else {
        panic!("minion")
    };
    facts.subtypes = Some(names.iter().map(|name| (*name).to_owned()).collect());
}

#[test]
fn subtype_filtered_protection_uses_recipient_characteristics_and_current_source_power() {
    let mut game = fixture(
        &json!({"alliedOnly":true,"recipientSubtype":"Faerie","preventsDamageFromUnitsWithPowerAtLeast":4}),
    );
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    let target = add_minion(&mut game, false, Seat::North, "D2", Region::Surface);
    set_subtypes(&mut game, &provider, &["Faerie"]);
    set_subtypes(&mut game, &target, &["Mortal"]);
    for (id, expected) in [(&provider, 0), (&target, 4)] {
        let status = game
            .unit_damage_status(UnitKind::Minion, Seat::North, id)
            .unwrap();
        let mut source = source(DamageOrigin::Other);
        source.current_power = 4;
        assert_eq!(
            Game::damage_after_prevention(&[(4, source)], status.prevention)
                .unwrap()
                .0,
            expected
        );
        source.current_power = 3;
        assert_eq!(
            Game::damage_after_prevention(&[(4, source)], status.prevention)
                .unwrap()
                .0,
            4
        );
    }
    set_subtypes(&mut game, &target, &["Faerie", "Mortal"]);
    game.position.units[1].temporary_modifiers.grant(
        TemporaryModifierKind::Silence,
        1,
        provider.clone(),
    );
    let protected = game
        .unit_damage_status(UnitKind::Minion, Seat::North, &target)
        .unwrap();
    let high_power = UnitDamageSource {
        current_power: 4,
        ..source(DamageOrigin::Other)
    };
    assert_eq!(
        Game::damage_after_prevention(&[(4, high_power)], protected.prevention)
            .unwrap()
            .0,
        0
    );
    let magic = UnitDamageSource {
        current_power: 0,
        ..source(DamageOrigin::Magic(ElementSet::only(Element::Fire)))
    };
    assert_eq!(
        Game::damage_after_prevention(&[(4, magic)], protected.prevention)
            .unwrap()
            .0,
        4
    );
    game.position.units[1].controller = Seat::South;
    let enemy = game
        .unit_damage_status(UnitKind::Minion, Seat::South, &target)
        .unwrap();
    assert_eq!(
        Game::damage_after_prevention(&[(4, high_power)], enemy.prevention)
            .unwrap()
            .0,
        4
    );
    let avatar = &game.position.players[0].avatar.card.instance_id;
    let status = game
        .unit_damage_status(UnitKind::Avatar, Seat::North, avatar)
        .unwrap();
    assert_eq!(
        Game::damage_after_prevention(&[(4, high_power)], status.prevention)
            .unwrap()
            .0,
        4
    );
}

#[test]
fn missing_recipient_subtypes_fail_closed_only_when_an_active_grant_can_apply() {
    let mut game = fixture(&json!({"recipientSubtype":"Faerie","takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    let target = add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
    set_subtypes(&mut game, &provider, &["Faerie"]);
    assert!(matches!(
        game.unit_damage_status(UnitKind::Minion, Seat::North, &target),
        Err(GameError::UnsupportedMechanic(_))
    ));
    game.position.units[0].disabled_until_damaged = true;
    assert!(
        game.unit_damage_status(UnitKind::Minion, Seat::North, &target)
            .is_ok()
    );
    game.position.units[0].disabled_until_damaged = false;
    game.position.units[1].location = Cell::parse("A1").unwrap();
    assert!(
        game.unit_damage_status(UnitKind::Minion, Seat::North, &target)
            .is_ok()
    );
}

#[test]
fn recipient_subtype_names_are_strictly_validated() {
    for name in [
        json!(null),
        json!(false),
        json!(4),
        json!(""),
        json!(" Faerie"),
        json!("Faerie\n"),
        json!("F\u{0000}ae"),
        json!("x".repeat(65)),
        json!("é".repeat(33)),
    ] {
        let card = json!({"cardType":"minion","attack":1,"defense":1,"manaCost":0,
            "thresholds":{"earth":0,"fire":0,"water":0,"air":0},"nearbyDamagePrevention":{"recipientSubtype":name,"takesLessDamage":1}});
        assert!(crate::facts::parse_card_definition("invalid", &card).is_err());
    }
}

#[test]
fn silenced_end_turn_abilities_do_not_trigger_before_silence_expires() {
    for silenced in [false, true] {
        let mut game = fixture(&json!({"takesLessDamage":1}));
        let id = add_minion(&mut game, false, Seat::North, "C3", Region::Surface);
        let card_id = game.position.units[0].card.card_id;
        let CardFacts::Minion(facts) =
            &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
        else {
            panic!("minion")
        };
        facts.untaps_at_end_of_controller_turn = true;
        facts.end_turn_stealth = Some(EndTurnStealth::Always);
        game.position.units[0].tapped = true;
        if silenced {
            game.position.units[0]
                .temporary_modifiers
                .grant(TemporaryModifierKind::Silence, 1, id);
        }
        game.finish_end_turn_cleanup(Seat::North, &mut OutcomeLog::Ignore)
            .unwrap();
        assert_eq!(game.position.units[0].tapped, silenced);
        assert_eq!(game.position.units[0].stealthed, !silenced);
    }
}

#[test]
fn silenced_provider_does_not_remove_enemy_stealth_until_its_ability_returns() {
    let mut game = fixture(&json!({"takesLessDamage":1}));
    let provider = add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
    add_minion(&mut game, false, Seat::South, "C3", Region::Surface);
    let card_id = game.position.units[0].card.card_id;
    let CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
    else {
        panic!("minion")
    };
    facts.nearby_enemies_permanently_lose_stealth = true;
    game.position.units[0]
        .temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, provider);
    game.position.units[1].stealthed = true;
    game.settle_nearby_enemy_stealth(&mut OutcomeLog::Ignore);
    assert!(game.position.units[1].stealthed);
    game.position.units[0]
        .temporary_modifiers
        .take(TemporaryModifierKind::Silence);
    game.settle_nearby_enemy_stealth(&mut OutcomeLog::Ignore);
    assert!(!game.position.units[1].stealthed);
}
