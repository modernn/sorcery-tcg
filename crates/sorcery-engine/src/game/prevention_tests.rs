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
            "chain":{"cardType":"magic","manaCost":0,"thresholds":zero,"damageChainNearbyUnits":true},
            "grid":{"cardType":"magic","manaCost":0,"thresholds":zero,"discardSiteAsAdditionalCost":true,"destroyTargetSite":true,"damageUnitsAboveAndBelowTargetSiteByManhattanDistance":[3,3,3,3,3]},
            "equipment":{"cardType":"artifact","manaCost":0,"thresholds":zero,"bearerPowerBonus":2,"bearerUnitStrike":{"damageBonus":1,"destroyAfterStrike":true}},
            "target":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"thresholds":zero}
        });
        m["decks"] = json!({
            "north":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":["provider","provider","provider","equipment"]},
            "south":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":["target","target","target","chain","grid"]}
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
    let player = &mut game.position.players[seat_index(hand_seat)];
    let card = [&mut player.hand_spellbook, &mut player.spellbook]
        .into_iter()
        .find_map(|zone| {
            zone.iter()
                .position(|card| {
                    game.rules.cards[usize::from(card.card_id.0)].id
                        == if provider { "provider" } else { "target" }
                })
                .map(|index| zone.remove(index))
        })
        .unwrap();
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
    game.settle_nearby_enemy_stealth(&mut OutcomeLog::Ignore)
        .expect("nearby Stealth settlement");
    assert!(game.position.units[1].stealthed);
    game.position.units[0]
        .temporary_modifiers
        .take(TemporaryModifierKind::Silence);
    game.settle_nearby_enemy_stealth(&mut OutcomeLog::Ignore)
        .expect("nearby Stealth settlement");
    assert!(!game.position.units[1].stealthed);
}

// Synthetic power dependencies: this fixture does not qualify any real card.
fn waking_power_group(seat: Seat, provider_defense: u8) -> (Game, IdentityHash, IdentityHash) {
    let mut game = fixture(&json!({"preventsDamageFrom":"magic"}));
    let provider = add_minion(&mut game, true, seat, "C3", Region::Surface);
    let target = add_minion(&mut game, false, seat, "C3", Region::Surface);
    let rules = Arc::get_mut(&mut game.rules).unwrap();
    for (unit, defense) in game.position.units.iter().zip([provider_defense, 3]) {
        let CardFacts::Minion(facts) = &mut rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            panic!("minion")
        };
        facts.attack = defense;
        facts.defense = defense;
    }
    game.position.units[0].disabled_until_damaged = true;
    game.position.active_seat = other_seat(seat);
    game.position.decision_seat = other_seat(seat);
    (game, provider, target)
}

#[test]
fn damage_groups_restore_all_power_before_capturing_any_casualty() {
    for seat in [Seat::North, Seat::South] {
        for reverse in [false, true] {
            for provider_defense in [2, 8] {
                let (mut game, provider, target) = waking_power_group(seat, provider_defense);
                if provider_defense == 2 {
                    let card_id = game.position.units[0].card.card_id;
                    let CardFacts::Minion(facts) =
                        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)]
                            .facts
                    else {
                        panic!("provider")
                    };
                    facts.deathrite_draw_site = true;
                }
                let mut targets = vec![
                    (provider.clone(), UnitKind::Minion, seat),
                    (target.clone(), UnitKind::Minion, seat),
                ];
                if reverse {
                    targets.reverse();
                }
                let mut silent = game.clone();
                let mut events = Vec::new();
                let casualties = game
                    .apply_area_damage(
                        targets.clone(),
                        3,
                        source(DamageOrigin::Other),
                        ("test-allocation", &provider),
                        &mut OutcomeLog::Record(&mut events),
                    )
                    .unwrap();
                let silent_casualties = silent
                    .apply_area_damage(
                        targets,
                        3,
                        source(DamageOrigin::Other),
                        ("test-allocation", &provider),
                        &mut OutcomeLog::Ignore,
                    )
                    .unwrap();
                assert_eq!(game.position, silent.position);
                assert_eq!(casualties.minions, silent_casualties.minions);
                assert_eq!(
                    casualties.minions,
                    if provider_defense == 2 {
                        vec![provider.clone()]
                    } else {
                        vec![]
                    }
                );
                assert!(!game.position.units[0].disabled_until_damaged);
                assert!(game.position.units.iter().all(|u| !u.death_marked));
                assert_eq!(
                    game.minion_current_stats(&game.position.units[1])
                        .unwrap()
                        .1,
                    4
                );
                game.begin_minion_deaths(
                    &casualties.minions,
                    &casualties.avatars,
                    Phase::Main,
                    other_seat(seat),
                    &mut OutcomeLog::Record(&mut events),
                )
                .unwrap();
                assert_eq!(
                    game.position.units.len(),
                    if provider_defense == 2 { 0 } else { 2 }
                );
                if provider_defense == 2 {
                    let draw_at = events
                        .iter()
                        .position(|(kind, p)| {
                            kind == "site-drawn" && p["sourceInstanceId"] == json!(provider)
                        })
                        .unwrap();
                    assert!(
                        draw_at
                            < events
                                .iter()
                                .position(|(kind, _)| kind == "minion-died")
                                .unwrap(),
                        "marked provider stays through its Deathrite before later power-loss deaths"
                    );
                    let deaths: Vec<_> = events
                        .iter()
                        .filter(|(kind, _)| kind == "minion-died")
                        .map(|(_, p)| p["instanceId"].clone())
                        .collect();
                    assert_eq!(
                        deaths,
                        [json!(provider), json!(target)],
                        "provider departure causes a later wave"
                    );
                }
            }
        }
    }
}

#[test]
fn effect_strikes_and_fight_windows_share_wake_settlement_without_retroactive_strikes() {
    for seat in [Seat::North, Seat::South] {
        for reverse in [false, true] {
            for fight in [false, true] {
                let (mut game, provider, target) = waking_power_group(seat, 8);
                let attacker_seat = other_seat(seat);
                let avatar = game.position.players[seat_index(attacker_seat)]
                    .avatar
                    .card
                    .instance_id
                    .clone();
                let mut targets = vec![
                    UnitTarget::Minion {
                        instance_id: provider.clone(),
                        seat,
                    },
                    UnitTarget::Minion {
                        instance_id: target.clone(),
                        seat,
                    },
                ];
                if reverse {
                    targets.reverse();
                }
                let mut silent = game.clone();
                let mut events = Vec::new();
                for (g, log) in [
                    (&mut game, OutcomeLog::Record(&mut events)),
                    (&mut silent, OutcomeLog::Ignore),
                ] {
                    let mut log = log;
                    if fight {
                        let pending = PendingCombat {
                            allocations: targets
                                .iter()
                                .map(|t| StrikeAllocation {
                                    amount: 3,
                                    target_instance_id: t.instance_id().clone(),
                                })
                                .collect(),
                            attacker_instance_id: avatar.clone(),
                            attacker_kind: UnitKind::Avatar,
                            attacking_seat: attacker_seat,
                            cell: Cell::parse("C3").unwrap(),
                            combatants: targets.clone(),
                            defenders: vec![],
                            original_target: None,
                            region: Region::Surface,
                            target_removed: false,
                        };
                        g.resolve_fight_window_with_damage(
                            &pending,
                            true,
                            &[provider.clone(), target.clone()],
                            None,
                            None,
                            &mut log,
                        )
                        .unwrap();
                        assert_eq!(
                            g.position.players[seat_index(attacker_seat)].avatar.life,
                            17,
                            "sleeping power provider cannot retaliate in its wake window"
                        );
                    } else {
                        let mut strike = g
                            .combatant_strike_stats(UnitKind::Avatar, attacker_seat, &avatar)
                            .unwrap();
                        strike.amount = 3;
                        g.resolve_effect_strikes(
                            &EffectStrikes {
                                source: UnitTarget::Avatar {
                                    instance_id: avatar.clone(),
                                    seat: attacker_seat,
                                },
                                targets: targets.clone(),
                                strike,
                                reveal_source: true,
                                return_phase: Phase::Main,
                                return_seat: attacker_seat,
                            },
                            &mut log,
                        )
                        .unwrap();
                    }
                }
                assert_eq!(game.position, silent.position);
                assert_eq!(game.position.units.len(), 2);
                assert!(
                    game.position
                        .units
                        .iter()
                        .all(|u| u.damage == 3 && !u.death_marked)
                );
                assert!(!events.iter().any(|(kind, _)| kind == "minion-died"));
            }
        }
    }
}

fn sleeping_own_power_fixture() -> (Game, IdentityHash) {
    let (mut game, _, target) = waking_power_group(Seat::North, 8);
    game.position.units.remove(0);
    let card_id = game.position.units[0].card.card_id;
    let rules = Arc::get_mut(&mut game.rules).unwrap();
    let CardFacts::Minion(facts) = &mut rules.cards[usize::from(card_id.0)].facts else {
        panic!("minion")
    };
    facts.attack = 2;
    facts.defense = 2;
    facts.gains_power_ranged_and_spellcaster_atop_tower = true;
    let card = game.position.players[0].hand_atlas.remove(0);
    let CardFacts::Site(site) = &mut rules.cards[usize::from(card.card_id.0)].facts else {
        panic!("site")
    };
    site.is_tower = true;
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(SitePosition {
        card,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    let unit = &mut game.position.units[0];
    unit.disabled_until_damaged = true;
    (game, target)
}

#[test]
fn single_damage_wake_uses_current_own_power_and_preserves_other_suppression() {
    for amount in [0, 1, 3, 4] {
        for suppression in [0, 1, 2] {
            let (mut game, target) = sleeping_own_power_fixture();
            let unit = &mut game.position.units[0];
            if suppression == 1 {
                unit.disable_effects.push(DisableEffect {
                    expires_at_seat: Seat::South,
                    source_instance_id: target.clone(),
                });
            }
            if suppression == 2 {
                unit.temporary_modifiers
                    .grant(TemporaryModifierKind::Silence, 1, target.clone());
            }
            game.damage_unit_and_settle_deaths(
                &(target, UnitKind::Minion, Seat::North),
                amount,
                source(DamageOrigin::Other),
                &mut OutcomeLog::Ignore,
            )
            .unwrap();
            let dead = amount >= if suppression == 0 { 4 } else { 2 };
            assert_eq!(game.position.units.is_empty(), dead);
            if !dead {
                assert_eq!(game.position.units[0].disabled_until_damaged, amount == 0);
                assert_eq!(
                    game.position.units[0].disable_effects.len(),
                    usize::from(suppression == 1)
                );
                assert_eq!(
                    game.minion_current_stats(&game.position.units[0])
                        .unwrap()
                        .1,
                    if amount > 0 && suppression == 0 { 4 } else { 2 }
                );
            }
        }
    }
}

#[test]
fn positive_lethal_survives_power_recovery_but_prevented_damage_does_not_wake() {
    for warded in [false, true] {
        let (mut game, provider, target) = waking_power_group(Seat::North, 8);
        game.position.units[0].warded = warded; // Synthetic later protection, not printed Ward on a disabled body.
        let status = game
            .unit_damage_status(UnitKind::Minion, Seat::North, &provider)
            .unwrap();
        let result = game
            .apply_simple_damage_with_status(
                UnitKind::Minion,
                Seat::North,
                &provider,
                1,
                UnitDamageSource {
                    lethal: true,
                    ..source(DamageOrigin::Other)
                },
                status,
                &mut OutcomeLog::Ignore,
            )
            .unwrap();
        let casualties = game
            .finish_damage_group(
                if result.lethal_damage_dealt {
                    vec![provider.clone()]
                } else {
                    vec![]
                },
                vec![],
            )
            .unwrap();
        assert_eq!(casualties.minions.contains(&provider), !warded);
        assert!(!casualties.minions.contains(&target));
        assert_eq!(game.position.units[0].disabled_until_damaged, warded);
        assert_eq!(game.position.units[0].damage, u16::from(!warded));
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the approved proof matrix keeps each simultaneous owner and its continuation assertions together"
)]
fn strike_control_reversion_and_consumption_finish_before_one_lethal_cohort() {
    for seat in [Seat::North, Seat::South] {
        for fight in [false, true] {
            for doubling in [false, true] {
                let (mut game, provider, victim) = waking_power_group(seat, 8);
                game.position.units[0].disabled_until_damaged = false;
                let striker = add_minion(&mut game, false, seat, "C3", Region::Surface);
                let striker_card = game.position.units[2].card.card_id;
                let CardFacts::Minion(facts) = &mut Arc::get_mut(&mut game.rules).unwrap().cards
                    [usize::from(striker_card.0)]
                .facts
                else {
                    panic!("minion")
                };
                facts.lethal = true;
                facts.deathrite_draw_site = true;
                facts.heals_controller_for_strike_damage = true;
                game.position.units[1].disabled_until_damaged = true;
                game.position.units[2].stealthed = true;
                game.position.units[2].damage = 5;
                game.position.units[2].carried_lance_count = u8::from(!doubling);
                if doubling {
                    game.position.units[2].temporary_modifiers.grant(
                        TemporaryModifierKind::NextStrikeDouble,
                        1,
                        provider.clone(),
                    );
                    let equipment = Arc::get_mut(&mut game.rules)
                        .unwrap()
                        .cards
                        .iter_mut()
                        .find(|c| c.id == "equipment")
                        .unwrap();
                    let CardFacts::Artifact(facts) = &mut equipment.facts else {
                        panic!("equipment")
                    };
                    facts.bearer_unit_strike.as_mut().unwrap().damage_bonus = 0;
                }
                game.position.temporary_controls.push(TemporaryControl {
                    expiry: TemporaryControlExpiry::UntilStealthLost,
                    instance_id: striker.clone(),
                    revert_to: other_seat(seat),
                    source_instance_id: provider,
                });
                let card_id = CardId(
                    u16::try_from(
                        game.rules
                            .cards
                            .iter()
                            .position(|c| c.id == "equipment")
                            .unwrap(),
                    )
                    .unwrap(),
                );
                let artifact_id =
                    crate::canonical::identity_hash(&json!({"synthetic-consumed-equipment":seat}))
                        .unwrap();
                game.position.artifacts.push(ArtifactPosition {
                    card: CardInstance {
                        card_id,
                        instance_id: artifact_id.clone(),
                        owner: seat,
                        realm_entry: 1,
                        source: CardSource::Spellbook,
                    },
                    placement: ArtifactPlacement::Carried {
                        bearer: UnitTarget::Minion {
                            instance_id: striker.clone(),
                            seat,
                        },
                        cell: None,
                    },
                });
                assert_eq!(
                    game.minion_current_stats(&game.position.units[2])
                        .unwrap()
                        .1,
                    6
                );
                let extra_victim =
                    add_minion(&mut game, false, other_seat(seat), "C3", Region::Surface);
                game.position.units[3].disabled_until_damaged = true;
                let strike = game
                    .combatant_strike_stats(UnitKind::Minion, seat, &striker)
                    .unwrap();
                let target = UnitTarget::Minion {
                    instance_id: victim.clone(),
                    seat,
                };
                game.position.players[seat_index(seat)].avatar.life = 10;
                let extra_target = UnitTarget::Minion {
                    instance_id: extra_victim.clone(),
                    seat: other_seat(seat),
                };
                let mut silent = game.clone();
                let mut events = Vec::new();
                for (g, log) in [
                    (&mut game, OutcomeLog::Record(&mut events)),
                    (&mut silent, OutcomeLog::Ignore),
                ] {
                    let mut log = log;
                    if fight {
                        let pending = PendingCombat {
                            allocations: vec![
                                StrikeAllocation {
                                    amount: 1,
                                    target_instance_id: victim.clone(),
                                },
                                StrikeAllocation {
                                    amount: 1,
                                    target_instance_id: extra_victim.clone(),
                                },
                            ],
                            attacker_instance_id: striker.clone(),
                            attacker_kind: UnitKind::Minion,
                            attacking_seat: seat,
                            cell: Cell::parse("C3").unwrap(),
                            combatants: vec![target.clone(), extra_target.clone()],
                            defenders: vec![],
                            original_target: None,
                            region: Region::Surface,
                            target_removed: false,
                        };
                        g.resolve_fight_window_with_damage(
                            &pending,
                            true,
                            std::slice::from_ref(&victim),
                            None,
                            Some(&[1, 1]),
                            &mut log,
                        )
                        .unwrap();
                    } else {
                        g.finish_effect_strikes(
                            &EffectStrikes {
                                source: UnitTarget::Minion {
                                    instance_id: striker.clone(),
                                    seat,
                                },
                                targets: vec![target.clone(), extra_target.clone()],
                                strike: strike.clone(),
                                reveal_source: true,
                                return_phase: Phase::Main,
                                return_seat: seat,
                            },
                            Some(&[1, 1]),
                            &mut log,
                        )
                        .unwrap();
                    }
                }
                assert_eq!(game.position, silent.position);
                assert_eq!(
                    game.position.players[seat_index(seat)].avatar.life,
                    12,
                    "healing retains captured attribution"
                );
                assert!(game.position.artifacts.is_empty());
                let pending = game.position.pending_deathrites.as_ref().unwrap();
                assert_eq!(
                    pending.marked.len(),
                    3,
                    "all final casualties are captured together"
                );
                for instance_id in [&striker, &victim, &extra_victim] {
                    assert_eq!(
                        pending
                            .marked
                            .iter()
                            .filter(|r| r.instance_id() == instance_id)
                            .count(),
                        1
                    );
                }
                assert!(
                    game.position
                        .units
                        .iter()
                        .find(|u| u.card.instance_id == striker)
                        .is_some_and(|u| u.controller == other_seat(seat)
                            && u.carried_lance_count == 0
                            && u.death_marked)
                );
                assert!(!events.iter().any(|(kind, _)| kind == "minion-died"));
                let order = game
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|a| matches!(a.descriptor, ActionDescriptor::OrderTriggers { .. }))
                    .unwrap();
                let owned = game.clone().apply_action_owned(&order).unwrap();
                let (resolved, _) = game.apply_action_recorded(&order).unwrap();
                silent.apply_action(&order).unwrap();
                assert_eq!(game.position, silent.position);
                assert_eq!(game.position, owned.position);
                events.extend(resolved);
                for instance_id in [&striker, &victim, &extra_victim] {
                    assert_eq!(
                        events
                            .iter()
                            .filter(|(kind, payload)| kind == "minion-died"
                                && payload["instanceId"] == json!(instance_id))
                            .count(),
                        1
                    );
                }
                assert!(
                    events
                        .iter()
                        .any(|(kind, payload)| kind == "minion-control-changed"
                            && payload["instanceId"] == json!(striker)
                            && payload["seat"] == json!(other_seat(seat)))
                );
                let event_at = |name: &str| events.iter().position(|(k, _)| k == name).unwrap();
                assert!(
                    event_at("minion-control-changed") < event_at("artifact-consumed-after-strike")
                );
                let consumed = if doubling {
                    "next-strike-double-consumed"
                } else {
                    "lance-broken"
                };
                assert!(event_at("artifact-consumed-after-strike") < event_at(consumed));
                assert_eq!(
                    events
                        .iter()
                        .filter(|(k, _)| k == "artifact-consumed-after-strike")
                        .count(),
                    1
                );
                assert!(event_at(consumed) < event_at("minion-died"));
            }
        }
    }
}

#[test]
fn issued_ranged_and_fixed_projectiles_use_restored_recipient_defense() {
    for fixed in [false, true] {
        let (mut game, target) = sleeping_own_power_fixture();
        game.position.players[1].domain_established = true;
        let shooter = add_minion(&mut game, true, Seat::South, "C2", Region::Surface);
        let card_id = game.position.units[1].card.card_id;
        let CardFacts::Minion(facts) =
            &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
        else {
            panic!("shooter")
        };
        facts.attack = 3;
        facts.other_nearby_allies_power_bonus = false;
        facts.ranged = !fixed;
        facts.tap_to_shoot_projectile_damage = fixed.then_some(3);
        game.position.units[1].summoning_sickness = false;
        game.position.players[0].avatar.location = Cell::parse("A1").unwrap();
        game.position.players[1].avatar.location = Cell::parse("E4").unwrap();
        let card = game.position.players[1].hand_atlas.remove(0);
        game.position.sites[Cell::parse("C2").unwrap().index()] = Some(SitePosition {
            card,
            controller: Seat::South,
            last_flight_turn: None,
            warded: false,
        });
        let action = game
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| match &a.descriptor {
                ActionDescriptor::ShootDamageProjectile {
                    shooter_instance_id,
                    hit: Some(hit),
                    ..
                } if fixed => *shooter_instance_id == shooter && *hit.instance_id() == target,
                ActionDescriptor::ShootProjectile {
                    shooter_instance_id,
                    hit: Some(hit),
                    ..
                } if !fixed => *shooter_instance_id == shooter && *hit.instance_id() == target,
                _ => false,
            })
            .unwrap();
        let mut silent = game.clone();
        let owned = game.clone().apply_action_owned(&action).unwrap();
        let (events, _) = game.apply_action_recorded(&action).unwrap();
        silent.apply_action(&action).unwrap();
        assert_eq!(game.position, silent.position);
        assert_eq!(game.position, owned.position);
        let recipient = game
            .position
            .units
            .iter()
            .find(|u| u.card.instance_id == target)
            .unwrap();
        assert_eq!(recipient.damage, 3);
        assert!(!recipient.disabled_until_damaged);
        assert_eq!(game.minion_current_stats(recipient).unwrap().1, 4);
        assert!(!events.iter().any(|(kind, _)| kind == "minion-died"));
    }
}

#[test]
fn deathrite_damage_and_artifact_exit_damage_finish_their_own_wake_group() {
    for deathrite in [false, true] {
        let (mut game, target) = sleeping_own_power_fixture();
        let source_id;
        if deathrite {
            source_id = add_minion(&mut game, true, Seat::South, "C3", Region::Surface);
            let card_id = game.position.units[1].card.card_id;
            let CardFacts::Minion(facts) =
                &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
            else {
                panic!("source")
            };
            facts.deathrite_damage_each_unit_here = Some(3);
            facts.other_nearby_allies_power_bonus = false;
        } else {
            source_id =
                crate::canonical::identity_hash(&json!({"synthetic-loose-source":true})).unwrap();
            let card_id = CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|c| c.id == "equipment")
                        .unwrap(),
                )
                .unwrap(),
            );
            game.position.artifacts.push(ArtifactPosition {
                card: CardInstance {
                    card_id,
                    instance_id: source_id.clone(),
                    owner: Seat::North,
                    realm_entry: 1,
                    source: CardSource::Spellbook,
                },
                placement: ArtifactPlacement::Loose {
                    location: Cell::parse("C3").unwrap(),
                    region: Region::Surface,
                },
            });
        }
        let mut silent = game.clone();
        let mut events = Vec::new();
        for (g, log) in [
            (&mut game, OutcomeLog::Record(&mut events)),
            (&mut silent, OutcomeLog::Ignore),
        ] {
            let mut log = log;
            if deathrite {
                g.begin_minion_deaths(
                    std::slice::from_ref(&source_id),
                    &[],
                    Phase::Main,
                    Seat::South,
                    &mut log,
                )
                .unwrap();
            } else {
                g.apply_destroy_own_artifact_at_location_for_area_damage(
                    Seat::North,
                    &source_id,
                    None,
                    3,
                    (&source_id, DamageOrigin::Other),
                    &mut log,
                )
                .unwrap();
            }
        }
        assert_eq!(game.position, silent.position);
        assert!(game.position.pending_deathrites.is_none());
        let recipient = game
            .position
            .units
            .iter()
            .find(|u| u.card.instance_id == target)
            .unwrap();
        assert_eq!(recipient.damage, 3);
        assert!(!recipient.disabled_until_damaged);
        assert!(
            !events
                .iter()
                .any(|(kind, p)| kind == "minion-died" && p["instanceId"] == json!(target))
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the approved proof matrix keeps each simultaneous owner and its continuation assertions together"
)]
fn issued_chain_roll_and_grid_damage_use_the_completed_wake_state() {
    for owner in ["chain", "roll", "grid"] {
        let (mut game, target) = sleeping_own_power_fixture();
        game.position.players[1].domain_established = true;
        game.position.players[1].mana = 8;
        let card = game.position.players[1].hand_atlas.remove(0);
        game.position.sites[Cell::parse("C2").unwrap().index()] = Some(SitePosition {
            card,
            controller: Seat::South,
            last_flight_turn: None,
            warded: false,
        });
        let step = |g: &mut Game, action: IssuedAction| {
            let mut silent = g.clone();
            let owned = g.clone().apply_action_owned(&action).unwrap();
            let (events, _) = g.apply_action_recorded(&action).unwrap();
            silent.apply_action(&action).unwrap();
            assert_eq!(g.position, silent.position);
            assert_eq!(g.position, owned.position);
            events
        };
        let events;
        if owner == "roll" {
            let target_card = game.position.units[0].card.card_id;
            let rules = Arc::get_mut(&mut game.rules).unwrap();
            let CardFacts::Minion(facts) = &mut rules.cards[usize::from(target_card.0)].facts
            else {
                panic!("target")
            };
            facts.attack = 3;
            facts.defense = 3;
            let artifact_card = CardId(
                u16::try_from(
                    rules
                        .cards
                        .iter()
                        .position(|c| c.id == "equipment")
                        .unwrap(),
                )
                .unwrap(),
            );
            let CardFacts::Artifact(facts) = &mut rules.cards[usize::from(artifact_card.0)].facts
            else {
                panic!("artifact")
            };
            facts.effect = ArtifactEffect::TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour;
            let artifact_id =
                crate::canonical::identity_hash(&json!({"synthetic-roll-source":true})).unwrap();
            game.position.artifacts.push(ArtifactPosition {
                card: CardInstance {
                    card_id: artifact_card,
                    instance_id: artifact_id.clone(),
                    owner: Seat::South,
                    realm_entry: 1,
                    source: CardSource::Spellbook,
                },
                placement: ArtifactPlacement::Loose {
                    location: Cell::parse("C2").unwrap(),
                    region: Region::Surface,
                },
            });
            let action = game.legal_actions().unwrap().into_iter().find(|a| matches!(&a.descriptor, ActionDescriptor::ActivateArtifactRollDamage { artifact_instance_id, path, .. } if *artifact_instance_id == artifact_id && path.iter().any(|p| p.cell == Cell::parse("C3").unwrap()))).unwrap();
            events = step(&mut game, action);
        } else {
            let name = if owner == "chain" { "chain" } else { "grid" };
            let player = &mut game.position.players[1];
            if let Some(index) = player
                .spellbook
                .iter()
                .position(|c| game.rules.cards[usize::from(c.card_id.0)].id == name)
            {
                let card = player.spellbook.remove(index);
                player.hand_spellbook.push(card);
            }
            let second_id =
                crate::canonical::identity_hash(&json!({"synthetic-second-chain-recipient":true}))
                    .unwrap();
            if owner == "chain" {
                let mut second = game.position.units[0].clone();
                second.card.instance_id = second_id.clone();
                game.position.units.push(second);
                let action = game.legal_actions().unwrap().into_iter().find(|a| matches!(&a.descriptor, ActionDescriptor::BeginChainMagic { card_id, target: t, .. } if card_id == "chain" && *t.instance_id() == target)).unwrap();
                step(&mut game, action);
                let extend = game.legal_actions().unwrap().into_iter().find(|a| matches!(&a.descriptor, ActionDescriptor::ExtendChainMagic { target: t } if *t.instance_id() == second_id)).unwrap();
                step(&mut game, extend);
                let resolve = game
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|a| matches!(a.descriptor, ActionDescriptor::ResolveChainMagic))
                    .unwrap();
                events = step(&mut game, resolve);
                assert!(
                    game.position
                        .units
                        .iter()
                        .all(|u| u.damage == 2 && !u.disabled_until_damaged)
                );
                assert_eq!(
                    events.iter().filter(|(k, _)| k == "magic-resolved").count(),
                    1
                );
            } else {
                let site_id = game.position.sites[Cell::parse("C2").unwrap().index()]
                    .as_ref()
                    .unwrap()
                    .card
                    .instance_id
                    .clone();
                let action = game.legal_actions().unwrap().into_iter().find(|a| matches!(&a.descriptor, ActionDescriptor::CastMagic { card_id, target_site_instance_id: Some(id), .. } if card_id == "grid" && *id == site_id)).unwrap();
                events = step(&mut game, action);
                assert!(game.position.sites[Cell::parse("C2").unwrap().index()].is_none());
            }
        }
        let recipient = game
            .position
            .units
            .iter()
            .find(|u| u.card.instance_id == target)
            .unwrap();
        assert!(!recipient.disabled_until_damaged);
        assert_eq!(
            recipient.damage,
            match owner {
                "chain" => 2,
                "roll" => 4,
                _ => 3,
            }
        );
        assert!(
            !events
                .iter()
                .any(|(kind, p)| kind == "minion-died" && p["instanceId"] == json!(target))
        );
    }
}

#[test]
#[ignore = "requires private source-guarded checkpoints via SORCERY_SLEEP_PROOF_CHECKPOINTS"]
fn private_damage_group_histories_match_record_silent_owned_and_replay() {
    let path = std::env::var("SORCERY_SLEEP_PROOF_CHECKPOINTS").unwrap();
    let data: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for (index, checkpoint) in data["checkpoints"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            crate::canonical::identity_hash(&checkpoint["manifest"]["cards"])
                .unwrap()
                .as_str(),
            data["sourceGuardedFactsHash"].as_str().unwrap()
        );
        let manifest = crate::canonical::canonical_json(&checkpoint["manifest"]).unwrap();
        let mut record = Game::from_manifest_json(&manifest).unwrap();
        let mut silent = record.clone();
        let mut owned = record.clone();
        let mut replay = record.clone();
        let mut event_hashes = Vec::new();
        for request in checkpoint["requests"].as_array().unwrap() {
            let actions = record.legal_actions().unwrap();
            assert_eq!(actions, silent.legal_actions().unwrap());
            assert_eq!(actions, owned.legal_actions().unwrap());
            assert_eq!(actions, replay.legal_actions().unwrap());
            let action = actions
                .into_iter()
                .find(|a| {
                    a.to_legal_action().unwrap().action_id.as_str()
                        == request["actionId"].as_str().unwrap()
                })
                .unwrap_or_else(|| panic!("unissued request in checkpoint {index}: {request}"));
            assert_eq!(json!(action.seat), request["seat"]);
            assert_eq!(
                action.to_legal_action().unwrap().state_version,
                request["stateVersion"].as_u64().unwrap()
            );
            let (events, random) = record.apply_action_recorded(&action).unwrap();
            let (again, again_random) = replay.apply_action_recorded(&action).unwrap();
            assert_eq!(events, again);
            assert_eq!(random, again_random);
            silent.apply_action(&action).unwrap();
            owned = owned.apply_action_owned(&action).unwrap();
            assert_eq!(record.position, silent.position);
            assert_eq!(record.position, owned.position);
            assert_eq!(record.position, replay.position);
            event_hashes.push(
                crate::canonical::identity_hash(&json!({"events":events,"random":random})).unwrap(),
            );
        }
        assert!(!event_hashes.is_empty());
        println!(
            "private damage history {index}: {} actions, receipts {}, state {}",
            event_hashes.len(),
            crate::canonical::identity_hash(&json!(event_hashes)).unwrap(),
            record.state_hash().unwrap()
        );
    }
}

#[test]
fn first_strike_wake_recovers_power_for_the_later_normal_window_only() {
    let (mut game, target) = sleeping_own_power_fixture();
    let attacker = add_minion(&mut game, true, Seat::South, "C3", Region::Surface);
    let card_id = game.position.units[1].card.card_id;
    let CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
    else {
        panic!("attacker")
    };
    facts.attack = 3;
    facts.defense = 5;
    facts.other_nearby_allies_power_bonus = false;
    game.position.units[1].temporary_modifiers.grant(
        TemporaryModifierKind::FirstStrike,
        1,
        attacker.clone(),
    );
    game.position.pending_combat = Some(PendingCombat {
        allocations: vec![StrikeAllocation {
            amount: 3,
            target_instance_id: target.clone(),
        }],
        attacker_instance_id: attacker.clone(),
        attacker_kind: UnitKind::Minion,
        attacking_seat: Seat::South,
        cell: Cell::parse("C3").unwrap(),
        combatants: vec![UnitTarget::Minion {
            instance_id: target.clone(),
            seat: Seat::North,
        }],
        defenders: vec![],
        original_target: None,
        region: Region::Surface,
        target_removed: false,
    });
    let mut silent = game.clone();
    let mut events = Vec::new();
    game.resolve_pending_fight(&mut OutcomeLog::Record(&mut events))
        .unwrap();
    silent
        .resolve_pending_fight(&mut OutcomeLog::Ignore)
        .unwrap();
    assert_eq!(game.position, silent.position);
    assert!(game.position.pending_combat.is_none());
    assert!(game.position.pending_deathrites.is_none());
    assert_eq!(game.position.units[0].damage, 3);
    assert_eq!(game.position.units[1].damage, 4);
    let damage: Vec<_> = events
        .iter()
        .filter(|(kind, _)| kind == "damage-dealt")
        .map(|(_, p)| (p["instanceId"].clone(), p["amount"].clone()))
        .collect();
    assert_eq!(
        damage,
        [(json!(target), json!(3)), (json!(attacker), json!(4))]
    );
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(game.position.decision_seat, Seat::South);
}

#[test]
fn prevention_is_snapshotted_for_the_whole_wake_group_in_either_seat() {
    for seat in [Seat::North, Seat::South] {
        for reverse in [false, true] {
            for asleep in [false, true] {
                let (mut game, provider, target) = waking_power_group(seat, 8);
                let card_id = game.position.units[0].card.card_id;
                let CardFacts::Minion(facts) =
                    &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
                else {
                    panic!("provider")
                };
                facts.nearby_damage_prevention.as_mut().unwrap().prevention =
                    DamagePrevention::TakesLessDamage(1);
                game.position.units[0].disabled_until_damaged = asleep;
                let mut targets = vec![
                    (provider.clone(), UnitKind::Minion, seat),
                    (target.clone(), UnitKind::Minion, seat),
                ];
                if reverse {
                    targets.reverse();
                }
                let casualties = game
                    .apply_area_damage(
                        targets,
                        3,
                        source(DamageOrigin::Other),
                        ("test-allocation", &provider),
                        &mut OutcomeLog::Ignore,
                    )
                    .unwrap();
                assert!(casualties.minions.is_empty());
                assert!(
                    game.position
                        .units
                        .iter()
                        .all(|u| u.damage == if asleep { 3 } else { 2 })
                );
                assert_eq!(
                    damage(
                        &game,
                        UnitKind::Minion,
                        seat,
                        &target,
                        DamageOrigin::Other,
                        3
                    ),
                    2
                );
            }
        }
    }
}

#[test]
fn zero_fully_prevented_and_partly_prevented_damage_have_distinct_sleep_endpoints() {
    for attempted in [0, 1, 2, 3] {
        let (mut game, target) = sleeping_own_power_fixture();
        add_minion(&mut game, true, Seat::North, "C3", Region::Surface);
        let card_id = game.position.units[1].card.card_id;
        let CardFacts::Minion(facts) =
            &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
        else {
            panic!("provider")
        };
        facts.other_nearby_allies_power_bonus = false;
        facts.nearby_damage_prevention.as_mut().unwrap().prevention =
            DamagePrevention::TakesLessDamage(1);
        game.damage_unit_and_settle_deaths(
            &(target, UnitKind::Minion, Seat::North),
            attempted,
            source(DamageOrigin::Other),
            &mut OutcomeLog::Ignore,
        )
        .unwrap();
        let recipient = &game.position.units[0];
        assert_eq!(recipient.damage, attempted.saturating_sub(1));
        assert_eq!(recipient.disabled_until_damaged, attempted <= 1);
        assert_eq!(
            game.minion_current_stats(recipient).unwrap().1,
            if attempted <= 1 { 2 } else { 4 }
        );
    }
}

// Synthetic combinations isolate live identities between the two fight windows.
fn first_strike_control_continuation_fixture(
    seat: Seat,
    attacker_reverts: bool,
) -> (Game, IdentityHash, IdentityHash) {
    let mut game = fixture(&json!({"alliedOnly":true,"takesLessDamage":1}));
    let enemy = other_seat(seat);
    let (attacker, defender, striker) = if attacker_reverts {
        add_minion(&mut game, true, seat, "C3", Region::Surface);
        let defender = add_minion(&mut game, true, enemy, "C3", Region::Surface);
        let attacker = add_minion(&mut game, false, seat, "C3", Region::Surface);
        let provider_card = game.position.units[1].card.card_id;
        let attacker_card = game.position.units[2].card.card_id;
        let tower = game.position.players[0].hand_atlas.remove(0);
        let rules = Arc::get_mut(&mut game.rules).unwrap();
        if let CardFacts::Minion(facts) = &mut rules.cards[usize::from(provider_card.0)].facts {
            facts.gains_power_ranged_and_spellcaster_atop_tower = true;
            facts.nearby_damage_prevention = None;
        }
        if let CardFacts::Minion(facts) = &mut rules.cards[usize::from(attacker_card.0)].facts {
            facts.attack = 1;
            facts.defense = 3;
        }
        if let CardFacts::Site(facts) = &mut rules.cards[usize::from(tower.card_id.0)].facts {
            facts.is_tower = true;
        }
        game.position.sites[Cell::parse("C3").unwrap().index()] = Some(SitePosition {
            card: tower,
            controller: enemy,
            last_flight_turn: None,
            warded: false,
        });
        game.position.units[1].disabled_until_damaged = true;
        game.position.units[2].damage = 3;
        (attacker.clone(), defender, attacker)
    } else {
        let site = game.position.players[0].hand_atlas.remove(0);
        game.position.sites[Cell::parse("C3").unwrap().index()] = Some(SitePosition {
            card: site,
            controller: seat,
            last_flight_turn: None,
            warded: false,
        });
        let attacker = add_minion(&mut game, false, seat, "C3", Region::Surface);
        let defender = add_minion(&mut game, false, enemy, "C3", Region::Surface);
        (attacker, defender.clone(), defender)
    };
    let unit = game
        .position
        .units
        .iter_mut()
        .find(|unit| unit.card.instance_id == striker)
        .unwrap();
    unit.stealthed = true;
    unit.carried_lance_count = 1;
    game.position.temporary_controls.push(TemporaryControl {
        expiry: TemporaryControlExpiry::UntilStealthLost,
        instance_id: striker.clone(),
        revert_to: if attacker_reverts { enemy } else { seat },
        source_instance_id: if attacker_reverts {
            defender.clone()
        } else {
            attacker.clone()
        },
    });
    game.position.active_seat = seat;
    game.position.decision_seat = seat;
    game.position.phase = Phase::Allocate;
    game.position.pending_combat = Some(PendingCombat {
        allocations: vec![],
        attacker_instance_id: attacker,
        attacker_kind: UnitKind::Minion,
        attacking_seat: seat,
        cell: Cell::parse("C3").unwrap(),
        combatants: vec![UnitTarget::Minion {
            instance_id: defender.clone(),
            seat: enemy,
        }],
        defenders: vec![],
        original_target: None,
        region: Region::Surface,
        target_removed: false,
    });
    (game, striker, defender)
}

#[test]
fn surviving_first_striker_control_reversion_refreshes_attacker_and_defender_in_later_window() {
    for seat in [Seat::North, Seat::South] {
        for attacker_reverts in [false, true] {
            let (mut record, striker, defender) =
                first_strike_control_continuation_fixture(seat, attacker_reverts);
            let mut silent = record.clone();
            let owned = record.clone();
            let mut replay = record.clone();
            let amount = if attacker_reverts { 3 } else { 1 };
            let action = record
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| {
                    matches!(
                        &action.descriptor,
                        ActionDescriptor::AllocateStrike { target_instance_id, amount: allocated }
                            if *target_instance_id == defender && *allocated == amount
                    )
                })
                .expect("issued final allocation");
            let (events, random) = record
                .apply_action_recorded(&action)
                .expect("later window resolves with live controllers");
            silent.apply_action(&action).unwrap();
            let owned = owned.apply_action_owned(&action).unwrap();
            let (again, again_random) = replay.apply_action_recorded(&action).unwrap();
            assert_eq!(events, again);
            assert_eq!(random, again_random);
            assert_eq!(record.position, silent.position);
            assert_eq!(record.position, owned.position);
            assert_eq!(record.position, replay.position);
            assert!(record.position.pending_combat.is_none());
            assert!(record.position.pending_deathrites.is_none());
            assert_eq!(record.position.phase, Phase::Main);
            assert_eq!(record.position.decision_seat, seat);
            let event_at = |name: &str| events.iter().position(|(kind, _)| kind == name).unwrap();
            assert!(event_at("minion-control-changed") < event_at("lance-broken"));
            assert_eq!(
                events
                    .iter()
                    .filter(|(kind, _)| kind == "lance-broken")
                    .count(),
                1
            );
            let damage: Vec<_> = events
                .iter()
                .filter(|(kind, _)| kind == "damage-dealt")
                .map(|(_, p)| (p["instanceId"].clone(), p["amount"].clone()))
                .collect();
            if attacker_reverts {
                assert_eq!(
                    damage,
                    [(json!(defender), json!(3)), (json!(striker), json!(4))]
                );
                let provider = record
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == defender)
                    .unwrap();
                assert!(!provider.disabled_until_damaged);
                assert_eq!(provider.damage, 3);
                assert_eq!(record.minion_current_stats(provider).unwrap().1, 4);
                assert!(event_at("lance-broken") < event_at("minion-died"));
                assert!(
                    !record
                        .position
                        .units
                        .iter()
                        .any(|unit| unit.card.instance_id == striker)
                );
            } else {
                let unit = record
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == striker)
                    .unwrap_or_else(|| panic!("missing reverted defender: {events:?}"));
                assert_eq!(unit.controller, seat);
                assert_eq!(unit.damage, 1);
                assert_eq!(unit.carried_lance_count, 0);
                assert_eq!(damage.len(), 2);
                assert_eq!(damage[0].1, json!(2));
                assert_eq!(damage[1], (json!(striker), json!(1)));
                assert!(!events.iter().any(|(kind, _)| kind == "minion-died"));
            }
        }
    }
}
