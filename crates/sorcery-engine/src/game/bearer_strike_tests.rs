//! Focused proofs for composable bearer unit-strike artifact facts.

use serde_json::json;

use super::{
    ArtifactPlacement, ArtifactPosition, CardId, CardInstance, CardSource, Cell, CombatTarget,
    Game, GameError, OutcomeLog, PendingCombat, Phase, Region, Seat, SummonPlacement,
    TemporaryModifierKind, UnitKind, UnitPosition, UnitTarget,
};
use crate::canonical::{IdentityHash, identity_hash};
use crate::synthetic::selfplay_manifest_with;

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"bearer-strike-test": label})).expect("test identity")
}

fn fixture() -> Game {
    let manifest = selfplay_manifest_with(9142, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "attack": 2,
            "cardType": "minion",
            "defense": 3,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["south-spell-1"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 3,
            "manaCost": 0,
            "preventsDamageFromUnitsWithPowerAtLeast": 1,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-49"] = json!({
            "bearerUnitStrike": {"damageBonus": 3, "firstStrike": true},
            "cardType": "artifact",
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-48"] = json!({
            "cardType": "magic",
            "effectProgram": {"effects": [{
                "op": "conjure-token", "token": "north-spell-50", "count": 1,
                "destination": "source"
            }]},
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-47"] = json!({
            "bearerUnitStrike": {"damageBonus": 1, "destroyAfterStrike": true},
            "cardType": "artifact",
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-50"] = json!({
            "bearerUnitStrike": {"damageBonus": 2, "destroyAfterStrike": true},
            "cardType": "artifact",
            "manaCost": null,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
            "token": true,
        });
        manifest["decks"]["north"]["spellbook"]
            .as_array_mut()
            .expect("north spellbook")
            .retain(|card| card != "north-spell-50");
    });
    let mut game = Game::from_manifest_json(&manifest).expect("bearer-strike fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game
}

fn card_id(game: &Game, name: &str) -> CardId {
    CardId(
        u16::try_from(
            game.rules
                .cards
                .iter()
                .position(|card| card.id == name)
                .expect("fixture card"),
        )
        .expect("card index"),
    )
}

fn minion(game: &Game, label: &str, seat: Seat) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: CardInstance {
            card_id: card_id(game, "north-spell-1"),
            instance_id: id(label),
            owner: seat,
            realm_entry: 1,
            source: CardSource::Spellbook,
        },
        controller: seat,
        lance_count: 0,
        location: Cell::parse("C3").expect("fixture cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("realm entry");
    unit.summoning_sickness = false;
    unit
}

fn south_minion(game: &Game, label: &str) -> UnitPosition {
    let mut unit = minion(game, label, Seat::South);
    unit.card.card_id = card_id(game, "south-spell-1");
    unit
}

fn carried_artifact(
    game: &Game,
    name: &str,
    label: &str,
    source: CardSource,
    bearer: &UnitPosition,
) -> ArtifactPosition {
    ArtifactPosition {
        card: CardInstance {
            card_id: card_id(game, name),
            instance_id: id(label),
            owner: Seat::North,
            realm_entry: 1,
            source,
        },
        placement: ArtifactPlacement::Carried {
            bearer: UnitTarget::Minion {
                instance_id: bearer.card.instance_id.clone(),
                seat: bearer.controller,
            },
            cell: None,
        },
    }
}

fn loose_artifact(game: &Game, name: &str, label: &str) -> ArtifactPosition {
    ArtifactPosition {
        card: CardInstance {
            card_id: card_id(game, name),
            instance_id: id(label),
            owner: Seat::North,
            realm_entry: 1,
            source: CardSource::Spellbook,
        },
        placement: ArtifactPlacement::Loose {
            location: Cell::parse("C3").expect("fixture cell"),
            region: Region::Surface,
        },
    }
}

fn pending(attacker: &UnitPosition) -> PendingCombat {
    PendingCombat {
        allocations: Vec::new(),
        attacker_instance_id: attacker.card.instance_id.clone(),
        attacker_kind: UnitKind::Minion,
        attacking_seat: attacker.controller,
        cell: attacker.location,
        combatants: Vec::new(),
        defenders: Vec::new(),
        original_target: None,
        region: attacker.region,
        target_removed: false,
    }
}

#[test]
fn carried_bonus_is_additive_and_loose_artifacts_are_ignored() {
    let mut game = fixture();
    let unit = minion(&game, "bearer", Seat::North);
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit.clone()];
    game.position.artifacts = vec![loose_artifact(&game, "north-spell-49", "loose")];
    let loose = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id)
        .expect("loose stats");
    assert_eq!(loose.amount, 2);
    assert_eq!(loose.current_power, 2);
    assert!(loose.consumed_artifacts.is_empty());

    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-49",
        "carried",
        CardSource::Spellbook,
        &unit,
    )];
    let carried = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id)
        .expect("carried stats");
    assert_eq!(carried.amount, 5);
    assert_eq!(carried.current_power, 2);
    assert_eq!(carried.additive_bonus, 3);
}

#[test]
fn token_and_nontoken_sources_stack_and_destroy_after_strike_consumes_only_marked_source() {
    let mut game = fixture();
    let unit = minion(&game, "bearer", Seat::North);
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit.clone()];
    game.position.artifacts = vec![
        carried_artifact(
            &game,
            "north-spell-49",
            "ordinary",
            CardSource::Spellbook,
            &unit,
        ),
        carried_artifact(&game, "north-spell-50", "token", CardSource::Token, &unit),
    ];
    let strike = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id)
        .expect("stacked stats");
    assert_eq!(strike.amount, 7);
    assert_eq!(strike.consumed_artifacts.len(), 1);
    let mut events = Vec::new();
    game.consume_strike_artifacts(&strike, &unit_id, &mut OutcomeLog::Record(&mut events))
        .expect("consume marked artifact");
    assert_eq!(game.position.artifacts.len(), 1);
    assert_eq!(
        game.position.artifacts[0].card.source,
        CardSource::Spellbook
    );
    assert!(
        events
            .iter()
            .any(|(kind, _)| kind == "artifact-consumed-after-strike")
    );
}

#[test]
fn carried_bonus_and_first_strike_apply_to_avatar_bearers() {
    let mut game = fixture();
    let avatar = &game.position.players[super::seat_index(Seat::North)].avatar;
    let avatar_id = avatar.card.instance_id.clone();
    game.position.artifacts.push(ArtifactPosition {
        card: CardInstance {
            card_id: card_id(&game, "north-spell-49"),
            instance_id: id("avatar-carried"),
            owner: Seat::North,
            realm_entry: 1,
            source: CardSource::Spellbook,
        },
        placement: ArtifactPlacement::Carried {
            bearer: UnitTarget::Avatar {
                instance_id: avatar_id.clone(),
                seat: Seat::North,
            },
            cell: None,
        },
    });
    let strike = game
        .combatant_strike_stats(UnitKind::Avatar, Seat::North, &avatar_id)
        .expect("avatar strike stats");
    assert_eq!(strike.amount, 4);
    assert!(
        game.combatant_strikes_first(UnitKind::Avatar, Seat::North, &avatar_id, true)
            .expect("avatar first strike")
    );
}

#[test]
fn prevented_strike_still_consumes_carried_artifact() {
    let mut game = fixture();
    let attacker = minion(&game, "attacker", Seat::North);
    let defender = south_minion(&game, "defender");
    let attacker_id = attacker.card.instance_id.clone();
    let defender_id = defender.card.instance_id.clone();
    game.position.units = vec![attacker, defender];
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-50",
        "consumed-token",
        CardSource::Token,
        &game.position.units[0].clone(),
    )];
    game.position.pending_combat = Some(pending(&game.position.units[0]));
    game.begin_fight(
        vec![UnitTarget::Minion {
            instance_id: defender_id.clone(),
            seat: Seat::South,
        }],
        &mut OutcomeLog::Ignore,
    )
    .expect("prevented fight");
    assert!(game.position.artifacts.is_empty());
    assert_eq!(
        game.position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == defender_id)
            .expect("defender survives")
            .damage,
        0
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == attacker_id)
    );
}

#[test]
fn simultaneous_targets_use_one_snapshot_and_consume_source_once() {
    let mut game = fixture();
    let attacker = minion(&game, "multi-attacker", Seat::North);
    let first = minion(&game, "multi-first", Seat::South);
    let second = minion(&game, "multi-second", Seat::South);
    let attacker_id = attacker.card.instance_id.clone();
    let target_ids = [
        first.card.instance_id.clone(),
        second.card.instance_id.clone(),
    ];
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-50",
        "multi-token",
        CardSource::Token,
        &attacker,
    )];
    game.position.units = vec![attacker, first, second];
    let mut events = Vec::new();
    game.apply_genesis_here_damage(&attacker_id, true, &mut OutcomeLog::Record(&mut events))
        .expect("simultaneous strikes");
    for target in target_ids {
        assert!(events.iter().any(|(kind, data)| kind == "damage-dealt"
            && data["instanceId"] == target.as_str()
            && data["amount"] == 4));
        assert!(
            !game
                .position
                .units
                .iter()
                .any(|unit| unit.card.instance_id == target)
        );
    }
    assert!(game.position.artifacts.is_empty());
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "artifact-consumed-after-strike")
            .count(),
        1
    );
    let last_damage = events
        .iter()
        .rposition(|(kind, _)| kind == "damage-dealt")
        .unwrap();
    let consumed = events
        .iter()
        .position(|(kind, _)| kind == "artifact-consumed-after-strike")
        .unwrap();
    assert!(consumed > last_damage);
}

#[test]
fn non_token_destroy_after_strike_enters_cemetery_and_stale_realm_source_is_not_consumed() {
    let mut game = fixture();
    let unit = minion(&game, "realm-bearer", Seat::North);
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit.clone()];
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-47",
        "ordinary-destroy",
        CardSource::Spellbook,
        &unit,
    )];
    let strike = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id)
        .expect("ordinary destroy stats");
    game.consume_strike_artifacts(&strike, &unit_id, &mut OutcomeLog::Ignore)
        .expect("ordinary destroy");
    assert!(game.position.artifacts.is_empty());
    assert!(
        game.position.players[super::seat_index(Seat::North)]
            .cemetery
            .iter()
            .any(|card| card.instance_id == id("ordinary-destroy"))
    );

    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-47",
        "stale-source",
        CardSource::Spellbook,
        &unit,
    )];
    let strike = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id)
        .expect("stale stats");
    game.position.artifacts[0].card.realm_entry += 1;
    game.consume_strike_artifacts(&strike, &unit_id, &mut OutcomeLog::Ignore)
        .expect("stale source does not fail");
    assert_eq!(game.position.artifacts.len(), 1);
}

#[test]
fn undefended_site_uses_base_power_and_does_not_consume_carried_source() {
    let mut game = fixture();
    let unit = minion(&game, "site-attacker", Seat::North);
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-50",
        "site-token",
        CardSource::Token,
        &unit,
    )];
    let mut combat = pending(&unit);
    let card = game.position.players[1].hand_atlas.remove(0);
    combat.original_target = Some(CombatTarget::Site {
        instance_id: card.instance_id.clone(),
        seat: Seat::South,
    });
    game.position.sites[unit.location.index()] = Some(super::SitePosition {
        card,
        controller: Seat::South,
        last_flight_turn: None,
        warded: false,
    });
    game.position.units = vec![unit];
    game.position.pending_combat = Some(combat);
    let life_before = game.position.players[1].avatar.life;
    let mut events = Vec::new();
    game.resolve_undefended_site_strike(&mut OutcomeLog::Record(&mut events))
        .expect("actual site strike");
    assert_eq!(game.position.artifacts.len(), 1);
    assert_eq!(game.position.players[1].avatar.life, life_before - 2);
    assert!(
        events
            .iter()
            .any(|(kind, data)| kind == "undefended-site-struck" && data["amount"] == 2)
    );
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "artifact-consumed-after-strike")
    );
}

#[test]
fn consuming_a_power_source_settles_its_bearer_death_in_the_same_window() {
    let mut game = fixture();
    let equipment = card_id(&game, "north-spell-47");
    let super::CardFacts::Artifact(facts) =
        &mut std::sync::Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(equipment.0)]
            .facts
    else {
        panic!("artifact")
    };
    facts.effect = super::ArtifactEffect::GrantsBearerPowerTwo;
    let mut attacker = minion(&game, "power-bearer", Seat::North);
    attacker.damage = 3;
    let defender = south_minion(&game, "immune-defender");
    let attacker_id = attacker.card.instance_id.clone();
    let defender_id = defender.card.instance_id.clone();
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-47",
        "power-source",
        CardSource::Spellbook,
        &attacker,
    )];
    let mut combat = pending(&attacker);
    combat.combatants.push(UnitTarget::Minion {
        instance_id: defender_id.clone(),
        seat: Seat::South,
    });
    combat.allocations.push(super::StrikeAllocation {
        amount: 5,
        target_instance_id: defender_id,
    });
    game.position.units = vec![attacker, defender];
    game.position.pending_combat = Some(combat.clone());
    assert_eq!(
        game.minion_current_stats(&game.position.units[0])
            .unwrap()
            .1,
        5
    );
    let interrupted = game
        .resolve_fight_window(&combat, true, &[], None, &mut OutcomeLog::Ignore)
        .expect("power-source strike");
    assert!(
        interrupted,
        "source removal causes a death even though the defender strikes no damage"
    );
    assert!(
        !game
            .position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == attacker_id)
    );
    assert!(
        game.position.players[0]
            .cemetery
            .iter()
            .any(|card| card.instance_id == attacker_id)
    );
    assert!(
        game.position.players[0]
            .cemetery
            .iter()
            .any(|card| card.instance_id == id("power-source"))
    );
}

#[test]
fn carried_first_strike_survives_bearer_silence_but_disabled_minion_does_not_strike() {
    let mut game = fixture();
    let mut unit = minion(&game, "bearer", Seat::North);
    unit.temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, id("silence"));
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit];
    let bearer = game.position.units[0].clone();
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-49",
        "carried",
        CardSource::Spellbook,
        &bearer,
    )];
    assert!(
        game.combatant_strikes_first(UnitKind::Minion, Seat::North, &unit_id, true)
            .expect("carried first strike")
    );

    game.position.units[0].disabled_until_damaged = true;
    let bearer = game.position.units[0].clone();
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-50",
        "disabled-carried-token",
        CardSource::Token,
        &bearer,
    )];
    let defender = south_minion(&game, "disabled-proof-defender");
    let defender_target = UnitTarget::Minion {
        instance_id: defender.card.instance_id.clone(),
        seat: Seat::South,
    };
    game.position.units.push(defender);
    let mut pending = pending(&game.position.units[0]);
    pending.combatants = vec![defender_target.clone()];
    pending.allocations = vec![super::StrikeAllocation {
        amount: 0,
        target_instance_id: defender_target.instance_id().clone(),
    }];
    game.position.pending_combat = Some(pending.clone());
    let mut events = Vec::new();
    game.resolve_fight_window(
        &pending,
        true,
        std::slice::from_ref(defender_target.instance_id()),
        None,
        &mut OutcomeLog::Record(&mut events),
    )
    .expect("disabled window");
    assert_eq!(game.position.artifacts.len(), 1);
}

#[test]
fn next_strike_doubling_with_bearer_bonus_is_explicitly_unsupported() {
    let mut game = fixture();
    let unit = minion(&game, "bearer", Seat::North);
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit.clone()];
    game.position.units[0].temporary_modifiers.grant(
        TemporaryModifierKind::NextStrikeDouble,
        1,
        id("double"),
    );
    game.position.artifacts = vec![carried_artifact(
        &game,
        "north-spell-49",
        "carried",
        CardSource::Spellbook,
        &unit,
    )];
    assert!(matches!(
        game.combatant_strike_stats(UnitKind::Minion, Seat::North, &unit_id),
        Err(GameError::UnsupportedMechanic(_))
    ));
}

#[test]
fn nearby_doubling_requires_ordering_only_at_an_affected_recipient() {
    let mut game = fixture();
    let mask_id = card_id(&game, "north-spell-49");
    let super::CardFacts::Artifact(facts) =
        &mut std::sync::Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(mask_id.0)].facts
    else {
        panic!("artifact")
    };
    facts.effect = super::ArtifactEffect::NearbyStrikesAgainstUnitsDealDoubleDamage;
    facts.bearer_unit_strike = None;
    let unit = minion(&game, "recipient", Seat::South);
    let unit_id = unit.card.instance_id.clone();
    game.position.units.push(unit);
    game.position
        .artifacts
        .push(loose_artifact(&game, "north-spell-49", "mask"));
    assert!(matches!(
        game.nearby_unit_strike_amount(5, 3, UnitKind::Minion, Seat::South, &unit_id),
        Err(GameError::UnsupportedMechanic(_))
    ));
    assert_eq!(
        game.nearby_unit_strike_amount(5, 0, UnitKind::Minion, Seat::South, &unit_id)
            .unwrap(),
        10
    );
    game.position.artifacts[0].placement = ArtifactPlacement::Loose {
        location: Cell::parse("A1").unwrap(),
        region: Region::Surface,
    };
    assert_eq!(
        game.nearby_unit_strike_amount(5, 3, UnitKind::Minion, Seat::South, &unit_id)
            .unwrap(),
        5
    );
    game.position.units[0].carried_lance_count = 1;
    game.position.units[0].temporary_modifiers.grant(
        TemporaryModifierKind::NextStrikeDouble,
        1,
        id("old-double"),
    );
    assert!(matches!(
        game.combatant_strike_stats(UnitKind::Minion, Seat::South, &unit_id),
        Err(GameError::UnsupportedMechanic(_))
    ));
}

#[test]
fn ranged_hit_consumes_equipment_but_a_miss_does_not() {
    let mut root = fixture();
    let shooter_card = card_id(&root, "north-spell-1");
    let super::CardFacts::Minion(facts) =
        &mut std::sync::Arc::get_mut(&mut root.rules).unwrap().cards[usize::from(shooter_card.0)]
            .facts
    else {
        panic!("minion")
    };
    facts.ranged = true;
    let shooter = minion(&root, "ranged-bearer", Seat::North);
    let mut target = minion(&root, "ranged-target", Seat::South);
    target.location = Cell::parse("C2").unwrap();
    let target_id = target.card.instance_id.clone();
    let shooter_id = shooter.card.instance_id.clone();
    for (seat, cell) in [
        (Seat::North, shooter.location),
        (Seat::South, target.location),
    ] {
        let card = root.position.players[super::seat_index(seat)]
            .hand_atlas
            .remove(0);
        root.position.sites[cell.index()] = Some(super::SitePosition {
            card,
            controller: seat,
            last_flight_turn: None,
            warded: false,
        });
        root.position.players[super::seat_index(seat)].domain_established = true;
    }
    root.position.artifacts.push(carried_artifact(
        &root,
        "north-spell-50",
        "ranged-token",
        CardSource::Token,
        &shooter,
    ));
    root.position.units = vec![shooter, target];
    for hit_expected in [false, true] {
        let mut game = root.clone();
        if !hit_expected {
            game.position.units[0].temporary_modifiers.grant(
                TemporaryModifierKind::NextStrikeDouble,
                1,
                id("miss-double"),
            );
        }
        let action = game.legal_actions().unwrap().into_iter().find(|action| {
            matches!(&action.descriptor, super::ActionDescriptor::ShootProjectile { shooter_instance_id, hit, .. }
                if *shooter_instance_id == shooter_id && if hit_expected {hit.as_ref().is_some_and(|target| *target.instance_id() == target_id)} else {hit.is_none()})
        }).expect("engine-issued ranged hit or miss");
        let (events, _) = game.apply_action_recorded(&action).expect("ranged action");
        assert_eq!(game.position.artifacts.is_empty(), hit_expected);
        assert_eq!(
            events
                .iter()
                .filter(|(kind, _)| kind == "artifact-consumed-after-strike")
                .count(),
            usize::from(hit_expected)
        );
        if hit_expected {
            assert!(events.iter().any(|(kind, data)| kind == "damage-dealt"
                && data["instanceId"] == target_id.as_str()
                && data["amount"] == 4));
        }
    }
}

#[test]
fn empty_genesis_strike_does_not_consume_or_require_damage_ordering() {
    let mut game = fixture();
    let mut unit = minion(&game, "empty-genesis", Seat::North);
    let unit_id = unit.card.instance_id.clone();
    unit.temporary_modifiers.grant(
        TemporaryModifierKind::NextStrikeDouble,
        1,
        id("empty-double"),
    );
    game.position.artifacts.push(carried_artifact(
        &game,
        "north-spell-50",
        "empty-token",
        CardSource::Token,
        &unit,
    ));
    game.position.units.push(unit);
    let mut events = Vec::new();
    game.apply_genesis_here_damage(&unit_id, true, &mut OutcomeLog::Record(&mut events))
        .expect("no strike without a target");
    assert!(events.is_empty());
    assert_eq!(game.position.artifacts.len(), 1);
    assert!(
        game.position.units[0]
            .temporary_modifiers
            .has(TemporaryModifierKind::NextStrikeDouble)
    );
}
