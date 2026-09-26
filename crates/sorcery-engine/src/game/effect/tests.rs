use std::sync::Arc;

use serde_json::json;

use super::super::ability::{CompiledAbility, Effect, UnitSet};
use super::super::{
    ActionDescriptor, CardId, CardInstance, CardSource, Cell, Game, OutcomeLog, Phase, Region,
    Seat, SummonPlacement, UnitDamageSource, UnitPosition, UnitTarget, seat_index,
};
use super::{AbilityEntry, EffectSource, RealmReference};
use crate::action::DeckZone;
use crate::board::Location;
use crate::canonical::identity_hash;
use crate::synthetic::selfplay_manifest_with;

fn fixture_game() -> Game {
    let manifest = selfplay_manifest_with(401, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "cardType": "magic",
            "damageTargetUnit": 1,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        for ordinal in 1..=2 {
            manifest["cards"][format!("south-spell-{ordinal}")] = json!({
                "attack": 1,
                "cardType": "minion",
                "deathriteDrawSpells": true,
                "defense": 1,
                "manaCost": 0,
                "preventsDamageFromUnitsWithPowerAtLeast": 4,
                "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
            });
        }
    });
    let mut game = Game::from_manifest_json(&manifest).expect("effect fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game
}

fn card_id(game: &Game, id: &str) -> CardId {
    CardId(
        u16::try_from(
            game.rules
                .cards
                .iter()
                .position(|card| card.id == id)
                .expect("fixture card"),
        )
        .expect("card index"),
    )
}

fn card(game: &Game, id: &str, owner: Seat, label: &str) -> CardInstance {
    CardInstance {
        card_id: card_id(game, id),
        instance_id: identity_hash(&json!({"effect-test": label})).expect("card identity"),
        owner,
        realm_entry: 0,
        source: CardSource::Spellbook,
    }
}

fn minion(game: &Game, id: &str, owner: Seat, label: &str, tapped: bool) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: card(game, id, owner, label),
        controller: owner,
        lance_count: 0,
        location: Cell::parse("C3").expect("fixture cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("fixture realm entry");
    unit.tapped = tapped;
    unit
}

fn source(
    label: &str,
    controller: Seat,
    power: u16,
    realm: Option<RealmReference>,
) -> EffectSource {
    EffectSource {
        instance_id: identity_hash(&json!({"effect-source": label})).expect("source identity"),
        owner: controller,
        controller,
        realm,
        actor: None,
        region: Region::Surface,
        cells: vec![Cell::parse("C3").expect("fixture cell")],
        damage: UnitDamageSource {
            current_power: power,
            lethal: false,
        },
    }
}

fn source_for_unit(
    unit: &UnitPosition,
    controller: Seat,
    power: u16,
    realm: Option<RealmReference>,
) -> EffectSource {
    let mut source = source("unit-source", controller, power, realm);
    source.instance_id = unit.card.instance_id.clone();
    source
}

fn install(game: &mut Game, effects: Vec<Effect>) -> CardId {
    let id = card_id(game, "north-spell-1");
    Arc::get_mut(&mut game.rules)
        .expect("fixture rules are uniquely owned")
        .cards[usize::from(id.0)]
    .abilities
    .magic = Some(CompiledAbility {
        effects: effects.into_boxed_slice(),
    });
    id
}

fn run_target_program(
    target_controller: Seat,
    warded: bool,
    power: u16,
) -> (Game, UnitPosition, Vec<(String, serde_json::Value)>) {
    let mut game = fixture_game();
    let target = minion(&game, "south-spell-1", target_controller, "target", true);
    let target_id = target.card.instance_id.clone();
    game.position.units = vec![target];
    game.position.units[0].warded = warded;
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic = install(
        &mut game,
        vec![
            Effect::Damage {
                recipients: UnitSet::Target,
                amount: 1,
            },
            Effect::Untap {
                recipients: UnitSet::Target,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let target = UnitTarget::Minion {
        seat: target_controller,
        instance_id: target_id,
    };
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("target-program", Seat::North, power, None),
            Some(&target),
            None,
            None,
        )
        .expect("target frame");
    let mut outcomes = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut outcomes))
        .expect("target frame runs");
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw + 1
    );
    let target = game.position.units[0].clone();
    (game, target, outcomes)
}

#[test]
fn target_protection_is_once_per_frame_but_independent_draw_and_friendly_untap_continue() {
    let (_opposing, target, events) = run_target_program(Seat::South, true, 1);
    assert_eq!(target.damage, 0);
    assert!(target.tapped);
    assert!(!target.warded);
    assert!(events.iter().any(|(kind, _)| kind == "spell-drawn"));
    assert!(!events.iter().any(|(kind, _)| kind == "minion-untapped"));

    let (friendly, target, _) = run_target_program(Seat::North, true, 1);
    assert_eq!(target.damage, 0);
    assert!(!target.tapped);
    assert!(!target.warded);
    assert_eq!(friendly.position.units.len(), 1);

    let (_, target, _) = run_target_program(Seat::North, false, 4);
    assert_eq!(target.damage, 0);
    assert!(!target.tapped);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one interrupted ability with normal and terminal branches"
)]
fn damage_cohort_orders_deathrites_then_resumes_draw_and_magic_cleanup() {
    let mut game = fixture_game();
    let caster = minion(&game, "south-spell-3", Seat::North, "caster", false);
    let caster_ref = RealmReference::from_card(&caster.card);
    let caster_id = caster.card.instance_id.clone();
    let first = minion(&game, "south-spell-1", Seat::North, "deathrite-a", false);
    let first_id = first.card.instance_id.clone();
    let second = minion(&game, "south-spell-2", Seat::North, "deathrite-b", false);
    let second_id = second.card.instance_id.clone();
    game.position.units = vec![caster, first, second];
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic_card = card(&game, "north-spell-1", Seat::North, "held-magic");
    let magic_id = magic_card.instance_id.clone();
    let magic = install(
        &mut game,
        vec![
            Effect::Damage {
                recipients: UnitSet::OtherUnitsHere,
                amount: 1,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source_for_unit(&game.position.units[0], Seat::North, 0, Some(caster_ref)),
            None,
            None,
            Some(magic_card),
        )
        .expect("cohort frame");
    let mut outcomes = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut outcomes))
        .expect("cohort damage");
    assert_eq!(game.position.phase, Phase::DeathriteOrder);
    assert!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .all(|card| card.instance_id != magic_id)
    );
    assert_eq!(game.position.units.len(), 1);
    assert_ne!(game.position.units[0].card.instance_id, first_id);
    assert_ne!(game.position.units[0].card.instance_id, second_id);
    let serialized = game.pending_deathrites_value(
        game.position
            .pending_deathrites
            .as_ref()
            .expect("paused effect"),
    );
    assert_eq!(serialized["continuation"]["kind"], "effect");
    assert_eq!(
        serialized["continuation"]["magic"]["instanceId"],
        json!(magic_id)
    );

    let order = game
        .legal_actions()
        .expect("deathrite actions")
        .into_iter()
        .find(|action| {
            matches!(
                &action.descriptor,
                ActionDescriptor::OrderDeathrites { source_instance_id }
                    if source_instance_id == &first_id || source_instance_id == &second_id
            )
        })
        .expect("deathrite order");
    let mut branch = game.clone();
    let mut exhausted = game.clone();
    exhausted.position.players[seat_index(Seat::North)]
        .spellbook
        .clear();
    let (terminal_events, _) = exhausted
        .apply_action_recorded(&order)
        .expect("terminal interruption");
    assert_eq!(exhausted.position.phase, Phase::Terminal);
    assert_eq!(
        exhausted.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .filter(|card| card.instance_id == magic_id)
            .count(),
        1
    );
    assert_eq!(
        terminal_events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert!(
        terminal_events
            .iter()
            .position(|(kind, _)| kind == "magic-resolved")
            .unwrap()
            < terminal_events
                .iter()
                .position(|(kind, _)| kind == "game-ended")
                .unwrap()
    );
    let (branch_events, _) = branch
        .apply_action_recorded(&order)
        .expect("cloned continuation");
    let (resolved, _) = game
        .apply_action_recorded(&order)
        .expect("resolve ordered cohort");
    assert_eq!(branch_events, resolved);
    assert_eq!(branch.position, game.position);
    assert_eq!(branch.state_hash().unwrap(), game.state_hash().unwrap());
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw + 3
    );
    assert_eq!(
        resolved
            .iter()
            .filter(|(kind, payload)| kind == "spell-drawn"
                && payload["sourceInstanceId"] == json!(caster_id))
            .count(),
        1
    );
    assert!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .any(|card| card.instance_id == magic_id)
    );
}

#[test]
fn exhausted_draw_finishes_held_magic_before_the_terminal_event() {
    let mut game = fixture_game();
    let magic_card = card(&game, "north-spell-1", Seat::North, "exhausted-draw");
    let magic_id = magic_card.instance_id.clone();
    let magic = install(
        &mut game,
        vec![Effect::Draw {
            zone: DeckZone::Spellbook,
            count: 1,
        }],
    );
    game.position.players[seat_index(Seat::North)]
        .spellbook
        .clear();
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("draw", Seat::North, 0, None),
            None,
            None,
            Some(magic_card),
        )
        .unwrap();
    let mut events = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    assert_eq!(game.position.phase, Phase::Terminal);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .filter(|card| card.instance_id == magic_id)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert!(
        events
            .iter()
            .position(|(kind, _)| kind == "magic-resolved")
            .unwrap()
            < events
                .iter()
                .position(|(kind, _)| kind == "game-ended")
                .unwrap()
    );
}

#[test]
fn other_units_query_includes_a_new_incarnation_of_the_old_source() {
    let mut game = fixture_game();
    let original = minion(&game, "south-spell-3", Seat::North, "query-source", false);
    let reference = RealmReference::from_card(&original.card);
    let effect_source = source_for_unit(&original, Seat::North, 0, Some(reference));
    let instance_id = original.card.instance_id.clone();
    game.position.units.push(original);
    let magic = install(
        &mut game,
        vec![Effect::Damage {
            recipients: UnitSet::OtherUnitsHere,
            amount: 1,
        }],
    );
    let mut frame = game
        .effect_frame(magic, AbilityEntry::Magic, effect_source, None, None, None)
        .unwrap();
    game.start_effect_frame(&mut frame, &mut OutcomeLog::Ignore);
    assert!(
        game.effect_recipients(&frame, UnitSet::OtherUnitsHere)
            .unwrap()
            .is_empty()
    );
    game.position.units[0].card.enter_realm().unwrap();
    let recipients = game
        .effect_recipients(&frame, UnitSet::OtherUnitsHere)
        .unwrap();
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0].0, instance_id);
}

#[test]
fn damage_uses_current_source_power_while_its_realm_incarnation_exists() {
    let mut game = fixture_game();
    let source = minion(&game, "south-spell-3", Seat::North, "changing-power", false);
    let source_ref = UnitTarget::Minion {
        seat: Seat::North,
        instance_id: source.card.instance_id.clone(),
    };
    let target = minion(&game, "south-spell-1", Seat::South, "power-immune", false);
    let target_ref = UnitTarget::Minion {
        seat: Seat::South,
        instance_id: target.card.instance_id.clone(),
    };
    game.position.units = vec![source, target];
    let source = game.unit_effect_source(&source_ref).unwrap();
    assert!(source.damage.current_power < 4);
    let source_card_id = card_id(&game, "south-spell-3");
    let super::super::CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(source_card_id.0)].facts
    else {
        unreachable!()
    };
    facts.attack = 6;
    facts.defense = 6;
    let magic = install(
        &mut game,
        vec![Effect::Damage {
            recipients: UnitSet::Target,
            amount: 1,
        }],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source,
            Some(&target_ref),
            None,
            None,
        )
        .unwrap();
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    let target = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == *target_ref.instance_id())
        .unwrap();
    assert_eq!(
        target.damage, 0,
        "immunity uses the source's power at damage resolution"
    );
}

#[test]
fn declaration_side_deaths_finish_before_the_queued_effect_starts() {
    let mut game = fixture_game();
    let first = minion(&game, "south-spell-1", Seat::North, "before-start-a", false);
    let second = minion(&game, "south-spell-2", Seat::North, "before-start-b", false);
    let deaths = vec![
        first.card.instance_id.clone(),
        second.card.instance_id.clone(),
    ];
    game.position.units = vec![first, second];
    let before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic = install(
        &mut game,
        vec![Effect::Draw {
            zone: DeckZone::Spellbook,
            count: 1,
        }],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("queued", Seat::North, 0, None),
            None,
            None,
            None,
        )
        .unwrap();
    game.begin_minion_deaths(
        &deaths,
        &[],
        Phase::Main,
        Seat::North,
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    assert_eq!(game.position.phase, Phase::DeathriteOrder);
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before
    );
    let pending = game.position.pending_deathrites.as_ref().unwrap();
    assert_eq!(
        game.pending_deathrites_value(pending)["continuation"]["started"],
        false
    );
    let action = game.legal_actions().unwrap().into_iter().next().unwrap();
    game.apply_action_recorded(&action).unwrap();
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before + 3
    );
}

#[test]
fn cast_keeps_declared_references_when_revealing_stealth_reverts_caster_control() {
    let mut game = fixture_game();
    game.position.players[seat_index(Seat::North)].domain_established = true;
    let caster_card_id = card_id(&game, "south-spell-3");
    let super::super::CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(caster_card_id.0)].facts
    else {
        unreachable!()
    };
    facts.spellcaster = true;
    facts.defense = 3;
    let mut caster = minion(&game, "south-spell-3", Seat::South, "stolen-caster", false);
    caster.controller = Seat::North;
    caster.stealthed = true;
    let caster_id = caster.card.instance_id.clone();
    game.position.units.push(caster);
    let mut site = card(&game, "south-site-1", Seat::South, "caster-site");
    site.enter_realm().unwrap();
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::South,
        last_flight_turn: None,
        warded: false,
    });
    game.position
        .temporary_controls
        .push(super::super::TemporaryControl {
            expiry: super::super::TemporaryControlExpiry::UntilStealthLost,
            instance_id: caster_id.clone(),
            revert_to: Seat::South,
            source_instance_id: caster_id.clone(),
        });
    let magic = card(&game, "north-spell-1", Seat::North, "stolen-caster-magic");
    let magic_id = magic.instance_id.clone();
    game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .push(magic);
    let action = game.legal_actions().unwrap().into_iter().find(|action| matches!(
        &action.descriptor,
        ActionDescriptor::CastMagic { card_instance_id, caster_instance_id, target: Some(target), .. }
            if *card_instance_id == magic_id && *caster_instance_id == caster_id && *target.instance_id() == caster_id
    )).expect("stolen caster can target itself with the declared spell");
    let (events, _) = game
        .apply_action_recorded(&action)
        .expect("cast survives control reversion");
    let caster = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == caster_id)
        .unwrap();
    assert_eq!(caster.controller, Seat::South);
    assert_eq!(caster.damage, 1);
    assert!(!caster.stealthed);
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .any(|card| card.instance_id == magic_id)
    );
}

#[test]
fn unit_binding_tracks_current_controller_and_rejects_target_reentry() {
    let mut game = fixture_game();
    let mut target = minion(
        &game,
        "south-spell-1",
        Seat::South,
        "controlled-target",
        true,
    );
    target.warded = true;
    let target_ref = UnitTarget::Minion {
        instance_id: target.card.instance_id.clone(),
        seat: Seat::South,
    };
    game.position.units.push(target);
    assert!(
        game.unit_reference(&UnitTarget::Minion {
            instance_id: target_ref.instance_id().clone(),
            seat: Seat::North,
        })
        .is_err(),
        "declaration must identify the current controller"
    );
    let magic = install(
        &mut game,
        vec![Effect::Untap {
            recipients: UnitSet::Target,
        }],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("control", Seat::North, 0, None),
            Some(&target_ref),
            None,
            None,
        )
        .unwrap();
    game.position.units[0].controller = Seat::North;
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    assert!(!game.position.units[0].tapped);
    assert!(
        game.position.units[0].warded,
        "current ally's Ward does not block Untap"
    );
    game.position.units[0].tapped = true;
    let target_ref = UnitTarget::Minion {
        seat: Seat::North,
        instance_id: target_ref.instance_id().clone(),
    };
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("reentry-target", Seat::North, 0, None),
            Some(&target_ref),
            None,
            None,
        )
        .unwrap();
    game.position.units[0].card.enter_realm().unwrap();
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    assert!(
        game.position.units[0].tapped,
        "a returned card is a new target object"
    );
}

#[test]
fn site_ward_protects_the_target_location_but_preserves_an_independent_draw() {
    let mut game = fixture_game();
    let location = Location {
        cell: Cell::parse("C3").unwrap(),
        region: Region::Surface,
    };
    let mut site_card = card(&game, "south-site-1", Seat::South, "warded-site");
    site_card.enter_realm().unwrap();
    game.position.sites[location.cell.index()] = Some(super::super::SitePosition {
        card: site_card,
        controller: Seat::South,
        last_flight_turn: None,
        warded: true,
    });
    let target = minion(
        &game,
        "south-spell-1",
        Seat::South,
        "protected-occupant",
        false,
    );
    game.position.units.push(target);
    let before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic = install(
        &mut game,
        vec![
            Effect::Damage {
                recipients: UnitSet::Location,
                amount: 1,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("location", Seat::North, 0, None),
            None,
            Some(location),
            None,
        )
        .unwrap();
    let mut events = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    assert_eq!(game.position.units.len(), 1);
    assert_eq!(game.position.units[0].damage, 0);
    assert!(
        !game.position.sites[location.cell.index()]
            .as_ref()
            .unwrap()
            .warded
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before + 1
    );
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "magic-damage-allocated")
    );
}

#[test]
fn realm_entry_prevents_rebinding_before_start_but_started_frame_survives_source_leave() {
    let mut game = fixture_game();
    let old = minion(&game, "south-spell-3", Seat::North, "reentry", true);
    let old_ref = RealmReference::from_card(&old.card);
    let old_id = old.card.instance_id.clone();
    let mut reentered = SummonPlacement {
        card: old.card.clone(),
        controller: Seat::North,
        lance_count: 0,
        location: old.location,
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    reentered.card.enter_realm().expect("reentry realm entry");
    assert_eq!(reentered.card.realm_entry, old.card.realm_entry + 1);
    game.position.units = vec![reentered];
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic = install(
        &mut game,
        vec![Effect::Draw {
            zone: DeckZone::Spellbook,
            count: 1,
        }],
    );
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source(
                "reentry-before-start",
                Seat::North,
                0,
                Some(old_ref.clone()),
            ),
            None,
            None,
            None,
        )
        .expect("pre-start frame");
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("stale frame skips");
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw
    );
    assert_eq!(game.position.units[0].card.instance_id, old_id);

    let mut game = fixture_game();
    let source_unit = minion(&game, "south-spell-3", Seat::North, "leave", false);
    let source_ref = RealmReference::from_card(&source_unit.card);
    let source_id = source_unit.card.instance_id.clone();
    game.position.units = vec![source_unit];
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let magic = install(
        &mut game,
        vec![
            Effect::Damage {
                recipients: UnitSet::Target,
                amount: 1,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let target = UnitTarget::Minion {
        seat: Seat::North,
        instance_id: source_id.clone(),
    };
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("leave-after-start", Seat::North, 0, Some(source_ref)),
            Some(&target),
            None,
            None,
        )
        .expect("post-start frame");
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("started frame continues");
    assert!(game.position.units.is_empty());
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw + 1
    );
}
