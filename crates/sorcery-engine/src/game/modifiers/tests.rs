use serde_json::{Value, json};

use super::{TemporaryModifierKind, TemporaryModifiers};
use crate::canonical::identity_hash;
use crate::game::{
    CardId, CardInstance, CardSource, Cell, Game, OutcomeLog, Phase, Region, Seat, SummonPlacement,
    UnitKind, UnitPosition, seat_index,
};
use crate::synthetic::selfplay_manifest_with;

fn source(label: &str) -> crate::canonical::IdentityHash {
    identity_hash(&json!({"temporary-modifier-test": label})).expect("source identity")
}

fn fixture_game() -> Game {
    let manifest = selfplay_manifest_with(7001, |_| {});
    let mut game = Game::from_manifest_json(&manifest).expect("modifier fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game
}

fn fixture_minion(game: &Game, card_name: &str, owner: Seat, label: &str) -> UnitPosition {
    let card_id = CardId(
        u16::try_from(
            game.rules
                .cards
                .iter()
                .position(|card| card.id == card_name)
                .expect("fixture minion"),
        )
        .expect("card index"),
    );
    let card = CardInstance {
        card_id,
        instance_id: source(label),
        owner,
        realm_entry: 0,
        source: CardSource::Spellbook,
    };
    let mut unit = SummonPlacement {
        card,
        controller: owner,
        lance_count: 0,
        location: Cell::parse("C3").expect("fixture cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("realm entry");
    unit
}

fn grant_all(modifiers: &mut TemporaryModifiers, label: &str) {
    for (index, kind) in [
        TemporaryModifierKind::Airborne,
        TemporaryModifierKind::Charge,
        TemporaryModifierKind::FirstStrike,
        TemporaryModifierKind::Lethal,
        TemporaryModifierKind::NextStrikeDouble,
        TemporaryModifierKind::Movement,
        TemporaryModifierKind::Power,
        TemporaryModifierKind::Ranged,
        TemporaryModifierKind::Silence,
    ]
    .into_iter()
    .enumerate()
    {
        modifiers.grant(kind, 1, source(&format!("{label}-{index}")));
    }
}

#[test]
fn duplicate_power_and_movement_survive_clone_and_authoritative_state() {
    let mut game = fixture_game();
    let north = &mut game.position.players[seat_index(Seat::North)]
        .avatar
        .temporary_modifiers;
    north.grant(TemporaryModifierKind::Power, 2, source("north-power-a"));
    north.grant(TemporaryModifierKind::Power, 2, source("north-power-b"));
    north.grant(TemporaryModifierKind::Movement, 1, source("north-move-a"));
    north.grant(TemporaryModifierKind::Movement, 1, source("north-move-b"));
    game.position.players[seat_index(Seat::South)]
        .avatar
        .temporary_modifiers
        .grant(TemporaryModifierKind::Power, 2, source("south-power"));

    let mut clone = game.clone();
    assert_eq!(game.position, clone.position);
    assert_eq!(
        game.state_hash().expect("state hash"),
        clone.state_hash().expect("clone hash")
    );
    let before_hash = game.state_hash().expect("state hash");
    clone.position.players[seat_index(Seat::North)]
        .avatar
        .temporary_modifiers
        .grant(TemporaryModifierKind::Movement, 1, source("north-move-c"));
    assert_ne!(before_hash, clone.state_hash().expect("changed state hash"));
    assert_eq!(
        game.avatar_current_stats(Seat::North).expect("power bonus"),
        (5, 5)
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .avatar
            .temporary_modifiers
            .amount(TemporaryModifierKind::Power),
        Ok(4)
    );
    assert_eq!(
        Game::avatar_basic_movement_steps(&game.position.players[seat_index(Seat::North)].avatar)
            .expect("movement steps"),
        3
    );
    let modifiers = &game.authoritative_state()["players"]["north"]["avatar"]["temporaryModifiers"];
    assert_eq!(modifiers.as_array().expect("modifier list").len(), 4);
    assert_eq!(modifiers[0]["kind"], Value::String("power".to_owned()));
}

#[test]
fn end_turn_expires_all_modifier_kinds_on_units_after_control_change() {
    let mut game = fixture_game();
    game.position.units = vec![
        fixture_minion(&game, "south-spell-1", Seat::North, "north-unit"),
        fixture_minion(&game, "south-spell-2", Seat::South, "south-unit"),
    ];
    grant_all(
        &mut game.position.players[seat_index(Seat::North)]
            .avatar
            .temporary_modifiers,
        "north-avatar",
    );
    grant_all(
        &mut game.position.players[seat_index(Seat::South)]
            .avatar
            .temporary_modifiers,
        "south-avatar",
    );
    grant_all(&mut game.position.units[0].temporary_modifiers, "north");
    grant_all(&mut game.position.units[1].temporary_modifiers, "south");
    game.position.units[0].controller = Seat::South;
    game.position.units[0].damage = 2;
    game.position.units[1].damage = 2;

    let mut events = Vec::new();
    let mut outcomes = OutcomeLog::Record(&mut events);
    game.finish_end_turn_cleanup(Seat::North, &mut outcomes)
        .expect("end-turn cleanup");

    assert!(
        game.position
            .units
            .iter()
            .all(|unit| unit.temporary_modifiers.is_empty())
    );
    assert_eq!(game.position.units.len(), 2);
    assert!(game.position.units.iter().all(|unit| unit.damage == 0));
    assert!(
        game.position
            .players
            .iter()
            .all(|player| player.avatar.temporary_modifiers.is_empty())
    );
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind.ends_with("-expired"))
            .count(),
        36
    );
}

#[test]
fn taking_next_strike_double_retains_other_modifier_kinds() {
    let mut game = fixture_game();
    let avatar_id = {
        let avatar = &mut game.position.players[seat_index(Seat::North)].avatar;
        avatar
            .temporary_modifiers
            .grant(TemporaryModifierKind::Power, 2, source("power"));
        avatar.temporary_modifiers.grant(
            TemporaryModifierKind::NextStrikeDouble,
            1,
            source("double"),
        );
        avatar.card.instance_id.clone()
    };
    let mut events = Vec::new();
    let mut outcomes = OutcomeLog::Record(&mut events);
    game.consume_next_strike_double(UnitKind::Avatar, Seat::North, &avatar_id, &mut outcomes)
        .expect("consume next-strike double");
    let modifiers = &game.position.players[seat_index(Seat::North)]
        .avatar
        .temporary_modifiers;
    assert!(!modifiers.has(TemporaryModifierKind::NextStrikeDouble));
    assert!(modifiers.has(TemporaryModifierKind::Power));
    assert_eq!(events.len(), 1);
}

#[test]
fn silence_suppresses_printed_and_granted_first_strike() {
    let manifest = selfplay_manifest_with(7002, |manifest| {
        manifest["cards"]["south-spell-1"]["strikesFirstWhileAttacking"] = json!(true);
    });
    let mut game = Game::from_manifest_json(&manifest).expect("first-strike fixture");
    game.position.phase = Phase::Main;
    let mut printed = fixture_minion(&game, "south-spell-1", Seat::South, "printed");
    let printed_id = printed.card.instance_id.clone();
    game.position.units = vec![printed.clone()];
    assert!(
        game.combatant_strikes_first(UnitKind::Minion, Seat::South, &printed_id, true)
            .expect("printed first strike before silence")
    );
    printed = game.position.units.pop().expect("printed unit");
    printed
        .temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, source("printed-silence"));
    game.position.units = vec![printed];
    assert!(
        !game
            .combatant_strikes_first(
                UnitKind::Minion,
                Seat::South,
                &game.position.units[0].card.instance_id,
                true,
            )
            .expect("printed first strike")
    );

    let granted = fixture_minion(&game, "south-spell-2", Seat::South, "granted");
    game.position.units[0] = granted;
    game.position.units[0].temporary_modifiers.grant(
        TemporaryModifierKind::FirstStrike,
        1,
        source("granted-first"),
    );
    assert!(
        game.combatant_strikes_first(
            UnitKind::Minion,
            Seat::South,
            &game.position.units[0].card.instance_id,
            true,
        )
        .expect("granted first strike before silence")
    );
    game.position.units[0].temporary_modifiers.grant(
        TemporaryModifierKind::Silence,
        1,
        source("granted-silence"),
    );
    assert!(
        !game
            .combatant_strikes_first(
                UnitKind::Minion,
                Seat::South,
                &game.position.units[0].card.instance_id,
                true,
            )
            .expect("granted first strike")
    );
    game.position.units[0]
        .temporary_modifiers
        .take(TemporaryModifierKind::Silence);
    assert!(
        game.combatant_strikes_first(
            UnitKind::Minion,
            Seat::South,
            &game.position.units[0].card.instance_id,
            true,
        )
        .expect("granted first strike after silence removal")
    );
}

fn fight(
    game: &mut Game,
    attacker: &crate::action::UnitTarget,
    defender: crate::action::UnitTarget,
) {
    game.position.pending_combat = Some(crate::game::PendingCombat {
        allocations: Vec::new(),
        attacker_kind: match attacker {
            crate::action::UnitTarget::Avatar { .. } => UnitKind::Avatar,
            crate::action::UnitTarget::Minion { .. } => UnitKind::Minion,
        },
        attacker_instance_id: attacker.instance_id().clone(),
        attacking_seat: attacker.seat(),
        cell: Cell::parse("C3").unwrap(),
        combatants: Vec::new(),
        defenders: Vec::new(),
        original_target: None,
        region: Region::Surface,
        target_removed: false,
    });
    game.begin_fight(vec![defender], &mut OutcomeLog::Ignore)
        .expect("fight");
}

#[test]
fn silenced_first_strike_trades_simultaneously_on_either_side_of_a_fight() {
    for attacking in [true, false] {
        for printed in [true, false] {
            for silenced in [true, false] {
                let manifest = selfplay_manifest_with(7003, |manifest| {
                    for id in ["south-spell-1", "south-spell-2"] {
                        manifest["cards"][id]["attack"] = json!(3);
                        manifest["cards"][id]["defense"] = json!(3);
                    }
                    if printed {
                        manifest["cards"]["south-spell-1"]["strikesFirstWhileAttacking"] =
                            json!(true);
                        manifest["cards"]["south-spell-1"]["strikesFirstWhileDefending"] =
                            json!(true);
                    }
                });
                let mut game = Game::from_manifest_json(&manifest).unwrap();
                let mut first = fixture_minion(&game, "south-spell-1", Seat::North, "first");
                let other = fixture_minion(&game, "south-spell-2", Seat::South, "other");
                if !printed {
                    first.temporary_modifiers.grant(
                        TemporaryModifierKind::FirstStrike,
                        1,
                        source("first-grant"),
                    );
                }
                if silenced {
                    first.temporary_modifiers.grant(
                        TemporaryModifierKind::Silence,
                        1,
                        source("silence"),
                    );
                }
                let first_target = crate::action::UnitTarget::Minion {
                    instance_id: first.card.instance_id.clone(),
                    seat: Seat::North,
                };
                let other_target = crate::action::UnitTarget::Minion {
                    instance_id: other.card.instance_id.clone(),
                    seat: Seat::South,
                };
                game.position.units = vec![first, other];
                if attacking {
                    fight(&mut game, &first_target, other_target);
                } else {
                    fight(&mut game, &other_target, first_target.clone());
                }
                assert_eq!(game.position.units.len(), usize::from(!silenced));
                if !silenced {
                    assert_eq!(
                        &game.position.units[0].card.instance_id,
                        first_target.instance_id()
                    );
                    assert_eq!(game.position.units[0].damage, 0);
                }
            }
        }
    }
}

#[test]
fn avatar_first_strike_prevents_return_damage_on_either_side_of_a_fight() {
    for attacking in [true, false] {
        let manifest = selfplay_manifest_with(7004, |manifest| {
            manifest["cards"]["south-spell-1"]["attack"] = json!(3);
            manifest["cards"]["south-spell-1"]["defense"] = json!(1);
        });
        let mut game = Game::from_manifest_json(&manifest).unwrap();
        let minion = fixture_minion(&game, "south-spell-1", Seat::South, "opponent");
        let minion_target = crate::action::UnitTarget::Minion {
            instance_id: minion.card.instance_id.clone(),
            seat: Seat::South,
        };
        game.position.units.push(minion);
        let avatar = &mut game.position.players[seat_index(Seat::North)].avatar;
        avatar.location = Cell::parse("C3").unwrap();
        avatar.temporary_modifiers.grant(
            TemporaryModifierKind::FirstStrike,
            1,
            source("avatar-first"),
        );
        let life = avatar.life;
        let avatar_target = crate::action::UnitTarget::Avatar {
            instance_id: avatar.card.instance_id.clone(),
            seat: Seat::North,
        };
        if attacking {
            fight(&mut game, &avatar_target, minion_target);
        } else {
            fight(&mut game, &minion_target, avatar_target);
        }
        assert!(game.position.units.is_empty());
        assert_eq!(
            game.position.players[seat_index(Seat::North)].avatar.life,
            life
        );
    }
}

#[test]
fn silence_suppresses_printed_and_granted_movement_without_removing_power() {
    let game = fixture_game();
    let mut unit = fixture_minion(&game, "south-spell-1", Seat::North, "movement-loss");
    let crate::facts::CardFacts::Minion(mut facts) = game.rules.cards
        [usize::from(unit.card.card_id.0)]
    .facts
    .clone() else {
        panic!("minion facts")
    };
    facts.movement_bonus = Some(2);
    unit.temporary_modifiers
        .grant(TemporaryModifierKind::Movement, 1, source("movement"));
    unit.temporary_modifiers
        .grant(TemporaryModifierKind::Power, 2, source("power"));
    assert_eq!(game.minion_basic_movement_steps(&unit, &facts).unwrap(), 4);
    unit.temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, source("silence"));
    assert_eq!(game.minion_basic_movement_steps(&unit, &facts).unwrap(), 1);
    assert_eq!(
        unit.temporary_modifiers
            .amount(TemporaryModifierKind::Power)
            .unwrap(),
        2
    );
    unit.temporary_modifiers
        .take(TemporaryModifierKind::Silence);
    assert_eq!(game.minion_basic_movement_steps(&unit, &facts).unwrap(), 4);
}
