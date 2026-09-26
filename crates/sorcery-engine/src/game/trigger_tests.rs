use serde_json::{Value, json};

use super::{
    ActionDescriptor, CardId, CardInstance, CardSource, Cell, DeferredMagicResolved, Game,
    OutcomeLog, Phase, Region, ResolutionContinuation, Seat, SitePosition, SummonPlacement,
    UnitPosition, UnitTarget, seat_index,
};
use crate::canonical::identity_hash;
use crate::synthetic::selfplay_manifest_with;

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

fn card(game: &Game, id: &str, owner: Seat, label: &str, source: CardSource) -> CardInstance {
    CardInstance {
        card_id: card_id(game, id),
        instance_id: identity_hash(&json!({ "trigger-test": label })).expect("card identity"),
        owner,
        realm_entry: 0,
        source,
    }
}

fn unit(game: &Game, id: &str, owner: Seat, label: &str, cell: &str) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: card(game, id, owner, label, CardSource::Spellbook),
        controller: owner,
        lance_count: 0,
        location: Cell::parse(cell).expect("unit cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("unit realm entry");
    unit
}

fn site(game: &Game, id: &str, owner: Seat, label: &str) -> SitePosition {
    let mut card = card(game, id, owner, label, CardSource::Atlas);
    card.enter_realm().expect("site realm entry");
    SitePosition {
        card,
        controller: owner,
        last_flight_turn: None,
        warded: false,
    }
}

fn manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "attack": 0,
            "cardType": "minion",
            "defense": 1,
            "genesisDrawSpells": 1,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-2"] = json!({
            "attack": 0,
            "cardType": "minion",
            "defense": 1,
            "genesisDamageEachOtherUnitHere": 1,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["south-spell-1"] = json!({
            "attack": 0,
            "cardType": "minion",
            "deathriteDrawSpells": true,
            "defense": 1,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["south-spell-2"] = manifest["cards"]["south-spell-1"].clone();
        manifest["cards"]["north-spell-3"] = json!({
            "cardType": "magic",
            "drawSpells": 1,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-4"] = json!({
            "attack": 0,
            "cardType": "minion",
            "defense": 1,
            "genesisDrawSite": true,
            "genesisHealController": 2,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-5"] = json!({
            "attack": 0,
            "cardType": "minion",
            "defense": 3,
            "genesisMayDamageTargetAdjacentUnit": 2,
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
    })
}

fn fixture(seed: u32) -> Game {
    let mut game = Game::from_manifest_json(&manifest(seed)).expect("trigger fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    for seat in [Seat::North, Seat::South] {
        game.position.players[seat_index(seat)].spellbook = (0..8)
            .map(|ordinal| {
                card(
                    &game,
                    if seat == Seat::North {
                        "north-spell-1"
                    } else {
                        "south-spell-1"
                    },
                    seat,
                    &format!("draw-{seat:?}-{ordinal}"),
                    CardSource::Spellbook,
                )
            })
            .collect();
    }
    game.position.sites[Cell::parse("C3").unwrap().index()] =
        Some(site(&game, "north-site-1", Seat::North, "north-c3"));
    game.position.sites[Cell::parse("C4").unwrap().index()] =
        Some(site(&game, "north-site-2", Seat::North, "north-c4"));
    game.position.units = Vec::new();
    game
}

fn trigger(game: &Game, seat: Seat, source: &UnitPosition) -> super::GenesisTrigger {
    game.genesis_trigger(seat, &source.card.instance_id, source.card.card_id)
        .expect("Genesis trigger")
        .expect("compiled Genesis")
}

fn order_action(game: &Game, instance_id: &str) -> super::IssuedAction {
    game.legal_actions()
        .expect("trigger order actions")
        .into_iter()
        .find(|action| {
            matches!(
                &action.descriptor,
                ActionDescriptor::OrderTriggers { source_instance_id }
                    if source_instance_id.as_str() == instance_id
            )
        })
        .expect("requested trigger order")
}

fn ability_action(
    game: &Game,
    source_instance_id: &str,
    target: Option<&super::UnitTarget>,
) -> super::IssuedAction {
    game.legal_actions()
        .expect("ability choice actions")
        .into_iter()
        .find(|action| {
            matches!(
                &action.descriptor,
                ActionDescriptor::ChooseAbility {
                    source_instance_id: actual_source,
                    target: actual,
                } if actual_source.as_str() == source_instance_id
                    && actual.as_ref() == target
            )
        })
        .expect("requested ability choice")
}

fn draw_sources(events: &[(String, Value)]) -> Vec<&str> {
    events
        .iter()
        .filter(|(kind, _)| kind == "spell-drawn")
        .map(|(_, payload)| payload["sourceInstanceId"].as_str().expect("draw source"))
        .collect()
}

#[test]
fn simultaneous_genesis_order_changes_effect_order() {
    let mut game = fixture(1101);
    let first = unit(&game, "north-spell-1", Seat::North, "draw-first", "C3");
    let second = unit(&game, "north-spell-1", Seat::North, "draw-second", "C4");
    let first_id = first.card.instance_id.clone();
    let second_id = second.card.instance_id.clone();
    game.position.units = vec![first.clone(), second.clone()];
    let first_trigger = trigger(&game, Seat::North, &first);
    let second_trigger = trigger(&game, Seat::North, &second);
    game.begin_genesis_triggers(vec![first_trigger, second_trigger], &mut OutcomeLog::Ignore)
        .expect("ordered Genesis batch");
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    let order = order_action(&game, second_id.as_str());
    let (events, _) = game
        .apply_action_recorded(&order)
        .expect("chosen trigger order");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        draw_sources(&events),
        [second_id.as_str(), first_id.as_str()]
    );
}

#[test]
fn queued_genesis_group_returns_to_the_resumed_phase() {
    let mut game = fixture(1108);
    let units: Vec<_> = (0..4)
        .map(|i| {
            unit(
                &game,
                "north-spell-1",
                Seat::North,
                &format!("queued-{i}"),
                "C3",
            )
        })
        .collect();
    game.position.units = units.clone();
    for group in units.chunks(2) {
        let triggers = group
            .iter()
            .map(|unit| trigger(&game, Seat::North, unit))
            .collect();
        game.begin_genesis_triggers(triggers, &mut OutcomeLog::Ignore)
            .unwrap();
    }
    let first = order_action(&game, units[0].card.instance_id.as_str());
    let (events, _) = game.apply_action_recorded(&first).unwrap();
    assert_eq!(draw_sources(&events).len(), 2);
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    let checkpoint = game.clone();
    let second = order_action(&game, units[2].card.instance_id.as_str());
    let result = game.apply_action_recorded(&second).unwrap();
    let mut replay = checkpoint;
    assert_eq!(result, replay.apply_action_recorded(&second).unwrap());
    assert_eq!(game.authoritative_state(), replay.authoritative_state());
    assert_eq!(game.position.phase, Phase::Main);
    assert!(game.position.pending_trigger_order.is_none());
    assert!(game.legal_actions().is_ok());
}

#[test]
fn genesis_death_chain_finishes_corpses_before_remaining_genesis() {
    let mut game = fixture(1102);
    let damaging = unit(&game, "north-spell-2", Seat::North, "damage-genesis", "C3");
    let remaining = unit(&game, "north-spell-1", Seat::North, "draw-genesis", "C4");
    let remaining_id = remaining.card.instance_id.clone();
    let deathrite_a = unit(&game, "south-spell-1", Seat::North, "deathrite-a", "C3");
    let deathrite_b = unit(&game, "south-spell-2", Seat::North, "deathrite-b", "C3");
    let damaging_id = damaging.card.instance_id.clone();
    game.position.units = vec![
        damaging.clone(),
        remaining.clone(),
        deathrite_a,
        deathrite_b,
    ];
    let damaging_trigger = trigger(&game, Seat::North, &damaging);
    let remaining_trigger = trigger(&game, Seat::North, &remaining);
    game.begin_genesis_triggers(
        vec![damaging_trigger, remaining_trigger],
        &mut OutcomeLog::Ignore,
    )
    .expect("Genesis trigger order");
    let first = order_action(&game, damaging_id.as_str());
    let (first_events, _) = game
        .apply_action_recorded(&first)
        .expect("first Genesis starts Deathrites");
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert!(game.position.pending_deathrites.is_some());
    assert!(!first_events.iter().any(|(kind, _)| kind == "spell-drawn"));
    let deathrite_order = game
        .legal_actions()
        .expect("nested Deathrite order")
        .into_iter()
        .find(|action| matches!(action.descriptor, ActionDescriptor::OrderTriggers { .. }))
        .expect("nested Deathrite trigger order");
    let (events, _) = game
        .apply_action_recorded(&deathrite_order)
        .expect("finish nested Deathrites and Genesis");
    assert_eq!(game.position.phase, Phase::Main);
    let last_death = events
        .iter()
        .rposition(|(kind, _)| kind == "minion-died")
        .expect("Deathrite corpses finalized");
    let remaining_draw = events
        .iter()
        .position(|(kind, payload)| {
            kind == "spell-drawn" && payload["sourceInstanceId"] == json!(remaining_id)
        })
        .expect("remaining Genesis draw");
    assert!(last_death < remaining_draw);
}

#[test]
fn non_active_genesis_resolves_before_active_genesis() {
    let mut game = fixture(1103);
    let active = unit(&game, "north-spell-1", Seat::North, "active-genesis", "C3");
    let non_active = unit(
        &game,
        "north-spell-1",
        Seat::South,
        "non-active-genesis",
        "C4",
    );
    let active_id = active.card.instance_id.clone();
    let non_active_id = non_active.card.instance_id.clone();
    game.position.units = vec![active.clone(), non_active.clone()];
    let active_trigger = trigger(&game, Seat::North, &active);
    let non_active_trigger = trigger(&game, Seat::South, &non_active);
    let mut events = Vec::new();
    game.begin_genesis_triggers(
        vec![active_trigger, non_active_trigger],
        &mut OutcomeLog::Record(&mut events),
    )
    .expect("mixed-controller Genesis");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        draw_sources(&events),
        [non_active_id.as_str(), active_id.as_str()]
    );
}

#[test]
fn controlled_genesis_draws_for_captured_controller() {
    let mut game = fixture(1104);
    let source = unit(
        &game,
        "north-spell-1",
        Seat::North,
        "captured-controller",
        "C3",
    );
    let source_id = source.card.instance_id.clone();
    game.position.units = vec![source.clone()];
    let source_trigger = trigger(&game, Seat::North, &source);
    let north_before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let south_before = game.position.players[seat_index(Seat::South)]
        .hand_spellbook
        .len();
    let mut controlled = source;
    controlled.controller = Seat::South;
    game.position.units = vec![controlled];
    game.begin_genesis_triggers(vec![source_trigger], &mut OutcomeLog::Ignore)
        .expect("captured Genesis controller");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        north_before + 1
    );
    assert_eq!(
        game.position.players[seat_index(Seat::South)]
            .hand_spellbook
            .len(),
        south_before
    );
    assert_eq!(
        game.position.units[0].card.instance_id, source_id,
        "the captured source still resolves its compiled ability"
    );
}

#[test]
fn reentered_source_does_not_resolve_stale_genesis_trigger() {
    let mut game = fixture(1105);
    let source = unit(&game, "north-spell-1", Seat::North, "stale-source", "C3");
    let source_id = source.card.instance_id.clone();
    game.position.units = vec![source.clone()];
    let source_trigger = trigger(&game, Seat::North, &source);
    let before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let mut reentered = source;
    reentered.card.enter_realm().expect("reentry");
    game.position.units = vec![reentered];
    game.begin_genesis_triggers(vec![source_trigger], &mut OutcomeLog::Ignore)
        .expect("stale Genesis is skipped");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before
    );
    assert_eq!(game.position.units[0].card.instance_id, source_id);
}

#[test]
fn simultaneous_uncompiled_genesis_rejects_without_partial_effects() {
    let mut game = fixture(1106);
    let first = unit(&game, "north-spell-4", Seat::North, "mixed-first", "C3");
    let second = unit(&game, "north-spell-4", Seat::North, "mixed-second", "C4");
    game.position.units = vec![first.clone(), second.clone()];
    let triggers = vec![
        trigger(&game, Seat::North, &first),
        trigger(&game, Seat::North, &second),
    ];
    let before = game.position.clone();
    let mut events = Vec::new();
    assert!(matches!(
        game.begin_genesis_triggers(triggers, &mut OutcomeLog::Record(&mut events)),
        Err(super::GameError::UnsupportedMechanic(_))
    ));
    assert_eq!(game.position, before);
    assert!(events.is_empty());
}

#[test]
fn singleton_targeted_genesis_declares_in_ability_choice_and_nap_resolves_first() {
    let mut game = fixture(1110);
    let active = unit(&game, "north-spell-5", Seat::North, "targeted-active", "C3");
    let non_active = unit(
        &game,
        "north-spell-5",
        Seat::South,
        "targeted-non-active",
        "C4",
    );
    let active_id = active.card.instance_id.clone();
    let non_active_id = non_active.card.instance_id.clone();
    game.position.units = vec![active.clone(), non_active.clone()];
    game.begin_genesis_triggers(
        vec![
            trigger(&game, Seat::North, &active),
            trigger(&game, Seat::South, &non_active),
        ],
        &mut OutcomeLog::Ignore,
    )
    .expect("targeted Genesis declarations");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    let active_target = UnitTarget::Minion {
        instance_id: non_active_id.clone(),
        seat: Seat::South,
    };
    let non_active_target = UnitTarget::Minion {
        instance_id: active_id,
        seat: Seat::North,
    };
    let active_choice = ability_action(
        &game,
        active.card.instance_id.as_str(),
        Some(&active_target),
    );
    game.apply_action_recorded(&active_choice)
        .expect("active Genesis declaration");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    let non_active_choice = ability_action(
        &game,
        non_active.card.instance_id.as_str(),
        Some(&non_active_target),
    );
    let (events, _) = game
        .apply_action_recorded(&non_active_choice)
        .expect("non-active Genesis declaration");
    let first_damage = events
        .iter()
        .find(|(kind, _)| kind == "genesis-damage-allocated")
        .expect("Genesis damage");
    assert_eq!(first_damage.1["sourceInstanceId"], json!(non_active_id));
}

#[test]
fn multiple_targeted_genesis_declare_every_source_before_resolution() {
    let mut game = fixture(1111);
    let first = unit(&game, "north-spell-5", Seat::North, "targeted-first", "C3");
    let second = unit(&game, "north-spell-5", Seat::North, "targeted-second", "C4");
    let first_id = first.card.instance_id.clone();
    let second_id = second.card.instance_id.clone();
    game.position.units = vec![first.clone(), second.clone()];
    game.begin_genesis_triggers(
        vec![
            trigger(&game, Seat::North, &first),
            trigger(&game, Seat::North, &second),
        ],
        &mut OutcomeLog::Ignore,
    )
    .expect("targeted Genesis declarations");
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert!(
        game.legal_actions()
            .expect("declaration actions")
            .iter()
            .any(|action| matches!(action.descriptor, ActionDescriptor::ChooseAbility { .. }))
    );
    assert_eq!(game.position.units.len(), 2);
    let first_target = UnitTarget::Minion {
        instance_id: second_id.clone(),
        seat: Seat::North,
    };
    let first_choice = ability_action(&game, first_id.as_str(), Some(&first_target));
    game.apply_action_recorded(&first_choice)
        .expect("first declaration");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    let second_target = UnitTarget::Minion {
        instance_id: first_id,
        seat: Seat::North,
    };
    let second_choice = ability_action(&game, second_id.as_str(), Some(&second_target));
    game.apply_action_recorded(&second_choice)
        .expect("second declaration");
}

#[test]
fn pending_targeted_genesis_clone_replays_identically() {
    let mut game = fixture(1112);
    let source = unit(&game, "north-spell-5", Seat::North, "clone-targeted", "C3");
    let target = unit(&game, "north-spell-1", Seat::South, "clone-target", "C4");
    game.position.units = vec![source.clone(), target.clone()];
    game.begin_genesis_triggers(
        vec![trigger(&game, Seat::North, &source)],
        &mut OutcomeLog::Ignore,
    )
    .expect("targeted Genesis declaration");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    let checkpoint = game.clone();
    let target = UnitTarget::Minion {
        instance_id: target.card.instance_id.clone(),
        seat: Seat::South,
    };
    let action = ability_action(&game, source.card.instance_id.as_str(), Some(&target));
    let result = game.apply_action_recorded(&action).expect("choice replay");
    let mut replay = checkpoint;
    assert_eq!(
        result,
        replay
            .apply_action_recorded(&action)
            .expect("choice replay")
    );
    assert_eq!(game.authoritative_state(), replay.authoritative_state());
}

#[test]
fn declining_optional_targeted_genesis_does_not_break_ward() {
    let mut game = fixture(1113);
    let source = unit(
        &game,
        "north-spell-5",
        Seat::North,
        "decline-targeted",
        "C3",
    );
    let mut target = unit(&game, "north-spell-1", Seat::South, "decline-target", "C4");
    target.warded = true;
    let target_id = target.card.instance_id.clone();
    game.position.units = vec![source.clone(), target];
    game.begin_genesis_triggers(
        vec![trigger(&game, Seat::North, &source)],
        &mut OutcomeLog::Ignore,
    )
    .expect("optional Genesis declaration");
    let action = ability_action(&game, source.card.instance_id.as_str(), None);
    game.apply_action_recorded(&action)
        .expect("decline optional Genesis");
    assert!(
        game.position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == target_id)
            .expect("target remains")
            .warded
    );
}

#[test]
fn terminal_genesis_skips_remaining_triggers_and_completes_held_magic_once() {
    let mut game = fixture(1107);
    let first = unit(&game, "north-spell-1", Seat::North, "terminal-first", "C3");
    let second = unit(&game, "north-spell-2", Seat::North, "terminal-second", "C3");
    let first_id = first.card.instance_id.clone();
    game.position.units = vec![first.clone(), second.clone()];
    game.position.players[seat_index(Seat::North)]
        .spellbook
        .clear();
    let triggers = vec![
        trigger(&game, Seat::North, &first),
        trigger(&game, Seat::North, &second),
    ];
    game.begin_genesis_triggers(triggers, &mut OutcomeLog::Ignore)
        .unwrap();
    let magic = card(
        &game,
        "north-spell-3",
        Seat::North,
        "held-magic",
        CardSource::Spellbook,
    );
    let magic_id = magic.instance_id.clone();
    game.continue_resolution(
        ResolutionContinuation::MagicResolved {
            resolution: DeferredMagicResolved {
                card_id: magic.card_id,
                instance_id: magic_id.clone(),
                owner: Seat::North,
            },
            held_card: Some(magic),
        },
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    let action = order_action(&game, first_id.as_str());
    let (events, _) = game.apply_action_recorded(&action).unwrap();
    assert_eq!(game.position.phase, Phase::Terminal);
    assert!(game.position.pending_trigger_order.is_none());
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .filter(|card| card.instance_id == magic_id)
            .count(),
        1
    );
    assert!(draw_sources(&events).is_empty());
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "genesis-damage-allocated")
    );
    assert_eq!(game.position.units.len(), 2);
}

#[test]
fn untargeted_trigger_can_be_ordered_before_an_undeclared_targeted_trigger() {
    let mut game = fixture(1114);
    let draw = unit(&game, "north-spell-1", Seat::North, "mixed-draw", "C3");
    let targeted = unit(&game, "north-spell-5", Seat::North, "mixed-targeted", "C3");
    game.position.units = vec![draw.clone(), targeted.clone()];
    let hand_size = game.position.players[0].hand_spellbook.len();
    game.begin_genesis_triggers(
        vec![
            trigger(&game, Seat::North, &draw),
            trigger(&game, Seat::North, &targeted),
        ],
        &mut OutcomeLog::Ignore,
    )
    .expect("mixed trigger group");
    let order = order_action(&game, draw.card.instance_id.as_str());
    let _declaration = ability_action(&game, targeted.card.instance_id.as_str(), None);
    let (events, _) = game
        .apply_action_recorded(&order)
        .expect("order draw first");
    assert!(
        draw_sources(&events).is_empty(),
        "no effect before all declarations"
    );
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    let decline = ability_action(&game, targeted.card.instance_id.as_str(), None);
    game.apply_action_recorded(&decline)
        .expect("finish declaring group");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(game.position.players[0].hand_spellbook.len(), hand_size + 1);
}
