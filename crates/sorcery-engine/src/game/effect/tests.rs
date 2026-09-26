use std::sync::Arc;

use serde_json::json;

use super::super::ability::{
    AbilityProgram, ControllerRelation, Effect, SelectionSpec, SpatialRelation, UnitArea,
    UnitCohort, UnitSet,
};
use super::super::{
    ActionDescriptor, CardId, CardInstance, CardSource, Cell, Game, GameError, OutcomeLog, Phase,
    Region, Seat, SummonPlacement, UnitDamageSource, UnitPosition, UnitTarget, seat_index,
};
use super::{AbilityEntry, EffectSource, RealmReference};
use crate::action::DeckZone;
use crate::board::Location;
use crate::canonical::identity_hash;
use crate::synthetic::selfplay_manifest_with;

const OTHER_UNITS_HERE: UnitSet = UnitSet::Query(UnitCohort {
    area: UnitArea::Source,
    kind: None,
    controller: ControllerRelation::Any,
    relation: None,
    exclude_source: true,
});
const AT_LOCATION: UnitSet = UnitSet::Query(UnitCohort {
    area: UnitArea::Location,
    kind: None,
    controller: ControllerRelation::Any,
    relation: None,
    exclude_source: false,
});

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
    let selection = effects.iter().find_map(|effect| match effect {
        Effect::Damage { recipients, .. }
        | Effect::Untap { recipients }
        | Effect::Grant { recipients, .. }
        | Effect::GiveStealth { recipients } => match recipients {
            UnitSet::Target => Some(SelectionSpec::Unit {
                kind: None,
                relation: SpatialRelation::Anywhere,
            }),
            UnitSet::Query(UnitCohort {
                area: UnitArea::Location,
                ..
            }) => Some(SelectionSpec::Location {
                relation: SpatialRelation::Anywhere,
            }),
            UnitSet::Query(_) | UnitSet::Chosen => None,
        },
        Effect::Draw { .. }
        | Effect::DrawCard
        | Effect::ChooseUnit(_)
        | Effect::SummonToken { .. } => None,
    });
    Arc::get_mut(&mut game.rules)
        .expect("fixture rules are uniquely owned")
        .cards[usize::from(id.0)]
    .abilities
    .magic = Some(Arc::new(AbilityProgram {
        optional_selection: false,
        selection,
        effects: effects.into_boxed_slice(),
    }));
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
fn shared_selection_separates_adjacent_nearby_regions_and_targeting() {
    use super::super::{UnitKind, UnitQuery};
    let mut game = fixture_game();
    let mut large = minion(&game, "south-spell-3", Seat::South, "large-selector", false);
    large.occupied_cells = Some(["C3", "C4", "D3", "D4"].map(|cell| Cell::parse(cell).unwrap()));
    large.stealthed = true;
    let large_id = large.card.instance_id.clone();
    let mut diagonal = minion(
        &game,
        "south-spell-3",
        Seat::South,
        "diagonal-selector",
        false,
    );
    diagonal.location = Cell::parse("B2").unwrap();
    diagonal.warded = true;
    let diagonal_id = diagonal.card.instance_id.clone();
    let mut underground = minion(
        &game,
        "south-spell-3",
        Seat::South,
        "underground-selector",
        false,
    );
    underground.region = Region::Underground;
    game.position.units = vec![diagonal, underground, large];
    let anchor = [Cell::parse("C3").unwrap()];
    let query = UnitQuery {
        region: Some(Region::Surface),
        cells: Some(&anchor),
        kind: Some(UnitKind::Minion),
        controller: None,
        exclude: None,
    };
    let adjacent = game.selected_units(query, SpatialRelation::Adjacent, None);
    assert_eq!(adjacent.len(), 1);
    assert_eq!(adjacent[0].instance_id(), &large_id);
    let nearby = game.selected_units(query, SpatialRelation::Nearby, None);
    assert_eq!(nearby.len(), 2);
    assert!(
        nearby
            .windows(2)
            .all(|pair| pair[0].instance_id() < pair[1].instance_id())
    );
    let targets = game.selected_units(query, SpatialRelation::Nearby, Some(Seat::North));
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].instance_id(), &diagonal_id);
    assert!(
        game.position.units[0].warded,
        "Ward is resolved after declaration"
    );
    assert_eq!(
        game.selected_units(query, SpatialRelation::Nearby, Some(Seat::South))
            .len(),
        2,
        "allied Stealth does not prevent targeting"
    );
}

#[test]
fn compiled_area_activation_requires_the_source_to_retain_its_ability() {
    let mut game = fixture_game();
    let id = card_id(&game, "south-spell-3");
    let definition = &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(id.0)];
    let super::super::CardFacts::Minion(facts) = &mut definition.facts else {
        unreachable!()
    };
    facts.tap_to_damage_each_unit_at_adjacent_location = true;
    definition.abilities = super::super::CompiledAbilities::from_facts(&definition.facts);
    let mut unit = minion(&game, "south-spell-3", Seat::North, "area-source", false);
    unit.summoning_sickness = false;
    let source_id = unit.card.instance_id.clone();
    game.position.units.push(unit);
    let mut site = card(&game, "south-site-1", Seat::North, "area-site");
    site.enter_realm().unwrap();
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    let mut actions = Vec::new();
    game.append_area_damage_actions(&mut actions, Seat::North);
    assert_eq!(actions.len(), 1, "the source can select its own location");
    game.position.units[0].temporary_modifiers.grant(
        super::super::TemporaryModifierKind::Silence,
        1,
        source_id,
    );
    actions.clear();
    game.append_area_damage_actions(&mut actions, Seat::North);
    assert!(actions.is_empty(), "silence removes the activated ability");
    game.position.units[0]
        .temporary_modifiers
        .take(super::super::TemporaryModifierKind::Silence);
    game.append_area_damage_actions(&mut actions, Seat::North);
    assert_eq!(actions.len(), 1, "the ability returns when silence expires");
}

#[test]
fn selection_is_revalidated_before_protection_but_not_after_an_effect_starts() {
    let mut game = fixture_game();
    let mut target = minion(
        &game,
        "south-spell-3",
        Seat::South,
        "moving-selection",
        true,
    );
    target.warded = true;
    let target_ref = UnitTarget::Minion {
        seat: Seat::South,
        instance_id: target.card.instance_id.clone(),
    };
    game.position.units.push(target);
    let magic = install(
        &mut game,
        vec![
            Effect::Untap {
                recipients: UnitSet::Target,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    Arc::make_mut(
        Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(magic.0)]
            .abilities
            .magic
            .as_mut()
            .unwrap(),
    )
    .selection = Some(SelectionSpec::Unit {
        kind: Some(super::super::UnitKind::Minion),
        relation: SpatialRelation::Nearby,
    });
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("range", Seat::North, 0, None),
            Some(&target_ref),
            None,
            None,
        )
        .unwrap();
    let mut started = frame.clone();
    game.position.units[0].location = Cell::parse("A1").unwrap();
    let before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    assert!(game.position.units[0].tapped);
    assert!(
        game.position.units[0].warded,
        "an invalid target does not consume Ward"
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before + 1
    );
    game.position.units[0].location = Cell::parse("C3").unwrap();
    game.position.units[0].warded = false;
    game.start_effect_frame(&mut started, &mut OutcomeLog::Ignore);
    game.position.units[0].location = Cell::parse("A1").unwrap();
    game.run_effect_frame(started, &mut OutcomeLog::Ignore)
        .unwrap();
    assert!(
        !game.position.units[0].tapped,
        "a started ability keeps its valid target binding"
    );
}

#[test]
fn magic_actor_departure_before_start_cancels_all_effects_and_releases_once() {
    let mut game = fixture_game();
    let caster = minion(
        &game,
        "south-spell-3",
        Seat::North,
        "departed-caster",
        false,
    );
    let caster_ref = RealmReference::from_card(&caster.card);
    let target = minion(&game, "south-spell-1", Seat::South, "departed-target", true);
    let target_id = target.card.instance_id.clone();
    let mut reentered = caster.clone();
    reentered.card.enter_realm().unwrap();
    game.position.units = vec![reentered, target];
    game.position.units[1].warded = true;

    let magic_card = card(&game, "north-spell-1", Seat::North, "departed-magic");
    let magic_id = magic_card.instance_id.clone();
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
    let mut effect_source = source("departed-source", Seat::North, 0, None);
    effect_source.instance_id = magic_id.clone();
    effect_source.actor = Some(caster_ref);
    let target_ref = UnitTarget::Minion {
        seat: Seat::South,
        instance_id: target_id.clone(),
    };
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            effect_source,
            Some(&target_ref),
            None,
            Some(magic_card),
        )
        .unwrap();
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let mut outcomes = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut outcomes))
        .unwrap();

    let target = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == target_id)
        .unwrap();
    assert_eq!(target.damage, 0);
    assert!(target.tapped);
    assert!(target.warded);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .filter(|card| card.instance_id == magic_id)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert!(outcomes.iter().all(|(kind, _)| kind == "magic-resolved"));
}

#[test]
fn magic_started_before_caster_departure_resumes_remaining_effects() {
    let mut game = fixture_game();
    let caster = minion(&game, "south-spell-3", Seat::North, "started-caster", false);
    let caster_ref = RealmReference::from_card(&caster.card);
    let caster_id = caster.card.instance_id.clone();
    let mut target = minion(&game, "south-spell-1", Seat::North, "started-target", true);
    let target_id = target.card.instance_id.clone();
    target.warded = true;
    game.position.units = vec![caster, target];

    let magic_card = card(&game, "north-spell-1", Seat::North, "started-magic");
    let magic_id = magic_card.instance_id.clone();
    let magic = install(
        &mut game,
        vec![
            Effect::Untap {
                recipients: UnitSet::Target,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let mut effect_source = source("started-source", Seat::North, 0, None);
    effect_source.instance_id = magic_id.clone();
    effect_source.actor = Some(caster_ref);
    let target_ref = UnitTarget::Minion {
        seat: Seat::North,
        instance_id: target_id.clone(),
    };
    let mut frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            effect_source,
            Some(&target_ref),
            None,
            Some(magic_card),
        )
        .unwrap();
    game.start_effect_frame(&mut frame, &mut OutcomeLog::Ignore);
    game.position
        .units
        .retain(|unit| unit.card.instance_id != caster_id);
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();

    let target = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == target_id)
        .unwrap();
    assert!(!target.tapped);
    assert!(target.warded);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw + 1
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .filter(|card| card.instance_id == magic_id)
            .count(),
        1
    );
}

#[test]
fn magic_moved_caster_with_spatial_selection_returns_explicit_unsupported_error() {
    let mut game = fixture_game();
    let caster = minion(&game, "south-spell-3", Seat::North, "moved-caster", false);
    let caster_ref = RealmReference::from_card(&caster.card);
    let target = minion(&game, "south-spell-1", Seat::South, "moved-target", true);
    let target_id = target.card.instance_id.clone();
    game.position.units = vec![caster, target];
    game.position.units[1].warded = true;

    let magic_card = card(&game, "north-spell-1", Seat::North, "moved-magic");
    let magic_id = magic_card.instance_id.clone();
    let magic = install(
        &mut game,
        vec![
            Effect::Untap {
                recipients: UnitSet::Target,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    Arc::make_mut(
        Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(magic.0)]
            .abilities
            .magic
            .as_mut()
            .unwrap(),
    )
    .selection = Some(SelectionSpec::Unit {
        kind: Some(super::super::UnitKind::Minion),
        relation: SpatialRelation::Nearby,
    });
    let mut effect_source = source("moved-source", Seat::North, 0, None);
    effect_source.instance_id = magic_id.clone();
    effect_source.actor = Some(caster_ref);
    let target_ref = UnitTarget::Minion {
        seat: Seat::South,
        instance_id: target_id.clone(),
    };
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            effect_source,
            Some(&target_ref),
            None,
            Some(magic_card),
        )
        .unwrap();
    game.position.units[0].location = Cell::parse("C4").unwrap();
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let mut outcomes = Vec::new();
    let error = game
        .run_effect_frame(frame, &mut OutcomeLog::Record(&mut outcomes))
        .unwrap_err();
    assert!(matches!(error, GameError::UnsupportedMechanic(_)));
    assert!(outcomes.is_empty());
    let target = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == target_id)
        .unwrap();
    assert!(target.tapped);
    assert!(target.warded);
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw
    );
    assert!(
        game.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .all(|card| card.instance_id != magic_id)
    );
}

#[test]
fn activated_area_effect_refreshes_moved_source_geometry_before_selection() {
    let mut game = fixture_game();
    let id = card_id(&game, "south-spell-3");
    let definition = &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(id.0)];
    let super::super::CardFacts::Minion(facts) = &mut definition.facts else {
        unreachable!()
    };
    facts.tap_to_damage_each_unit_at_adjacent_location = true;
    facts.defense = 5;
    definition.abilities = super::super::CompiledAbilities::from_facts(&definition.facts);

    let mut source_unit = minion(
        &game,
        "south-spell-3",
        Seat::North,
        "moved-area-source",
        false,
    );
    source_unit.location = Cell::parse("A1").unwrap();
    source_unit.summoning_sickness = false;
    let mut target = minion(
        &game,
        "south-spell-3",
        Seat::South,
        "moved-area-target",
        false,
    );
    target.location = Cell::parse("C4").unwrap();
    let target_id = target.card.instance_id.clone();
    game.position.units = vec![source_unit, target];
    let mut site = card(&game, "south-site-1", Seat::South, "moved-area-site");
    site.enter_realm().unwrap();
    game.position.sites[Cell::parse("C4").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::South,
        last_flight_turn: None,
        warded: false,
    });

    let mut effect_source = source_for_unit(
        &game.position.units[0],
        Seat::North,
        0,
        Some(RealmReference::from_card(&game.position.units[0].card)),
    );
    effect_source.cells = vec![Cell::parse("C3").unwrap()];
    let frame = game
        .effect_frame(
            id,
            AbilityEntry::Activated,
            effect_source,
            None,
            Some(Location {
                cell: Cell::parse("C4").unwrap(),
                region: Region::Surface,
            }),
            None,
        )
        .unwrap();
    let mut outcomes = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut outcomes))
        .unwrap();

    let target = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == target_id)
        .unwrap();
    assert_eq!(
        target.damage, 0,
        "the declared location is no longer adjacent"
    );
    assert!(!outcomes.iter().any(|(kind, payload)| {
        kind == "area-damage-allocated" && payload["targetInstanceId"] == json!(target_id)
    }));
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
                recipients: OTHER_UNITS_HERE,
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
    assert_eq!(game.position.phase, Phase::TriggerOrder);
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
                ActionDescriptor::OrderTriggers { source_instance_id }
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
fn departed_source_cannot_anchor_a_query_but_its_new_incarnation_can_join_realm_cohorts() {
    let mut game = fixture_game();
    let original = minion(&game, "south-spell-3", Seat::North, "query-source", false);
    let reference = RealmReference::from_card(&original.card);
    let effect_source = source_for_unit(&original, Seat::North, 0, Some(reference));
    let instance_id = original.card.instance_id.clone();
    game.position.units.push(original);
    let magic = install(
        &mut game,
        vec![Effect::Damage {
            recipients: OTHER_UNITS_HERE,
            amount: 1,
        }],
    );
    let mut frame = game
        .effect_frame(magic, AbilityEntry::Magic, effect_source, None, None, None)
        .unwrap();
    game.start_effect_frame(&mut frame, &mut OutcomeLog::Ignore);
    assert!(
        game.effect_recipients(&frame, OTHER_UNITS_HERE)
            .unwrap()
            .is_empty()
    );
    game.position.units[0].card.enter_realm().unwrap();
    assert!(matches!(
        game.effect_recipients(&frame, OTHER_UNITS_HERE),
        Err(GameError::UnsupportedMechanic(_))
    ));
    let recipients = game
        .effect_recipients(
            &frame,
            UnitSet::Query(UnitCohort {
                area: UnitArea::Realm {
                    region: Some(Region::Surface),
                },
                relation: None,
                kind: Some(super::super::UnitKind::Minion),
                controller: ControllerRelation::Any,
                exclude_source: true,
            }),
        )
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
    assert_eq!(game.position.phase, Phase::TriggerOrder);
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
                recipients: AT_LOCATION,
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

#[test]
fn ordinary_unit_choice_resumes_one_effect_and_survives_checkpoint_cloning() {
    use super::super::ability::UnitChoiceSpec;

    let mut game = fixture_game();
    let mut first = minion(&game, "south-spell-1", Seat::North, "choice-first", true);
    first.warded = true;
    first.stealthed = true;
    let second = minion(&game, "south-spell-2", Seat::North, "choice-second", true);
    let selected = UnitTarget::Minion {
        instance_id: first.card.instance_id.clone(),
        seat: Seat::North,
    };
    game.position.units = vec![first, second];
    let source = game.unit_effect_source(&selected).expect("live source");
    let source_id = source.instance_id.clone();
    let id = install(
        &mut game,
        vec![
            Effect::ChooseUnit(UnitChoiceSpec {
                exclude_source: false,
                kind: None,
                relation: SpatialRelation::Adjacent,
                allied_only: true,
                optional: false,
            }),
            Effect::Untap {
                recipients: UnitSet::Chosen,
            },
        ],
    );
    let frame = game
        .effect_frame(id, AbilityEntry::Magic, source, None, None, None)
        .expect("ordinary choice program");
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("pause for choice");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    assert!(game.authoritative_state()["pendingAbilityChoice"].is_object());
    let mut actions = Vec::new();
    game.append_ability_choice_actions(&mut actions)
        .expect("choices");
    assert!(actions.iter().all(|action| matches!(
        action.descriptor,
        ActionDescriptor::ChooseAbility {
            target: Some(_),
            ..
        }
    )));
    assert!(actions.iter().any(|action| matches!(&action.descriptor,
        ActionDescriptor::ChooseAbility { target: Some(target), .. } if target == &selected)));
    let mut branch = game.clone();
    let mut outcomes = Vec::new();
    let mut branch_outcomes = Vec::new();
    game.apply_ability_choice_action(
        Seat::North,
        &source_id,
        Some(&selected),
        &mut OutcomeLog::Record(&mut outcomes),
    )
    .expect("choose self");
    branch
        .apply_ability_choice_action(
            Seat::North,
            &source_id,
            Some(&selected),
            &mut OutcomeLog::Record(&mut branch_outcomes),
        )
        .expect("same branch choice");
    assert_eq!(game.authoritative_state(), branch.authoritative_state());
    assert_eq!(outcomes, branch_outcomes);
    assert_eq!(game.position.phase, Phase::Main);
    assert!(!game.position.units[0].tapped);
    assert!(game.position.units[0].warded);
    assert!(game.position.units[0].stealthed);
    assert!(
        game.position.units[1].tapped,
        "only the selected ally untaps"
    );
    assert!(game.position.pending_ability_choice.is_none());
}

#[test]
fn emptied_choice_after_source_departure_resumes_independent_effects() {
    use super::super::ability::UnitChoiceSpec;
    let mut game = fixture_game();
    let unit = minion(&game, "south-spell-1", Seat::North, "only-choice", true);
    let selected = UnitTarget::Minion {
        instance_id: unit.card.instance_id.clone(),
        seat: Seat::North,
    };
    game.position.units = vec![unit];
    let source = game.unit_effect_source(&selected).expect("source");
    let id = install(
        &mut game,
        vec![
            Effect::ChooseUnit(UnitChoiceSpec {
                exclude_source: false,
                kind: Some(super::super::UnitKind::Minion),
                relation: SpatialRelation::Adjacent,
                allied_only: true,
                optional: false,
            }),
            Effect::Untap {
                recipients: UnitSet::Chosen,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let frame = game
        .effect_frame(id, AbilityEntry::Magic, source, None, None, None)
        .expect("frame");
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("pending choice");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    // Simulate settlement removing the sole eligible unit while the effect is interrupted.
    game.position.units.clear();
    let before = game.position.players[0].hand_spellbook.len();
    game.resume_empty_ability_choice(&mut OutcomeLog::Ignore)
        .expect("empty choice resumes");
    assert_eq!(game.position.phase, Phase::Main);
    assert!(game.position.pending_ability_choice.is_none());
    assert_eq!(game.position.players[0].hand_spellbook.len(), before + 1);
}

#[test]
fn ordinary_anywhere_choice_crosses_regions_but_targets_and_nearby_do_not() {
    use crate::ability::{UnitChoiceSpec, UnitKind};
    let mut game = fixture_game();
    let mut hidden = minion(
        &game,
        "south-spell-1",
        Seat::North,
        "underground-ally",
        false,
    );
    hidden.region = Region::Underground;
    hidden.stealthed = true;
    hidden.warded = true;
    let selected = UnitTarget::Minion {
        instance_id: hidden.card.instance_id.clone(),
        seat: Seat::North,
    };
    game.position.units.push(hidden);
    let source = source("surface-source", Seat::North, 0, None);
    let spec = UnitChoiceSpec {
        exclude_source: false,
        kind: Some(UnitKind::Minion),
        relation: SpatialRelation::Anywhere,
        allied_only: true,
        optional: false,
    };
    assert_eq!(game.ability_unit_choices(&source, spec), vec![selected]);
    assert!(
        game.ability_unit_choices(
            &source,
            UnitChoiceSpec {
                relation: SpatialRelation::Nearby,
                ..spec
            }
        )
        .is_empty()
    );
    assert!(
        game.selection_choices(
            Seat::North,
            Region::Surface,
            &source.cells,
            Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                relation: SpatialRelation::Anywhere
            })
        )
        .is_empty()
    );
}

#[test]
fn empty_second_choice_does_not_reuse_first_chosen_unit() {
    use crate::ability::{TemporaryModifierKind, UnitChoiceSpec, UnitKind};
    let mut game = fixture_game();
    let selected = UnitTarget::Avatar {
        instance_id: game.position.players[0].avatar.card.instance_id.clone(),
        seat: Seat::North,
    };
    let source = source("successive-choice", Seat::North, 0, None);
    let source_id = source.instance_id.clone();
    let spec = UnitChoiceSpec {
        exclude_source: false,
        kind: Some(UnitKind::Avatar),
        relation: SpatialRelation::Anywhere,
        allied_only: true,
        optional: false,
    };
    let id = install(
        &mut game,
        vec![
            Effect::ChooseUnit(spec),
            Effect::Grant {
                duration: crate::ability::EffectDuration::ThisTurn,
                recipients: UnitSet::Chosen,
                modifier: TemporaryModifierKind::Movement,
                amount: 1,
            },
            Effect::ChooseUnit(UnitChoiceSpec {
                kind: Some(UnitKind::Minion),
                ..spec
            }),
            Effect::Grant {
                duration: crate::ability::EffectDuration::ThisTurn,
                recipients: UnitSet::Chosen,
                modifier: TemporaryModifierKind::Power,
                amount: 2,
            },
        ],
    );
    let frame = game
        .effect_frame(id, AbilityEntry::Magic, source, None, None, None)
        .unwrap();
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    game.apply_ability_choice_action(
        Seat::North,
        &source_id,
        Some(&selected),
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    let modifiers = &game.position.players[0].avatar.temporary_modifiers;
    assert!(modifiers.has(TemporaryModifierKind::Movement));
    assert!(!modifiers.has(TemporaryModifierKind::Power));
    assert!(game.position.pending_ability_choice.is_none());
}

#[test]
#[allow(clippy::too_many_lines)]
fn authored_cohorts_share_controller_region_footprint_and_source_filters() {
    let mut game = fixture_game();
    let ally = minion(&game, "south-spell-3", Seat::North, "cohort-ally", false);
    let mut lower = minion(&game, "south-spell-3", Seat::North, "cohort-lower", false);
    lower.region = Region::Underwater;
    let mut void = minion(&game, "south-spell-3", Seat::North, "cohort-void", false);
    void.region = Region::Void;
    let mut remote = minion(&game, "south-spell-3", Seat::North, "cohort-remote", false);
    remote.location = Cell::parse("A1").unwrap();
    let mut enemy = minion(&game, "south-spell-3", Seat::South, "cohort-large", false);
    enemy.occupied_cells = Some(["C3", "C4", "D3", "D4"].map(|c| Cell::parse(c).unwrap()));
    enemy.warded = true;
    enemy.stealthed = true;
    let ids = [&ally, &lower, &void, &remote, &enemy].map(|u| u.card.instance_id.clone());
    game.position.players[0].avatar.location = Cell::parse("C3").unwrap();
    let avatar_id = game.position.players[0].avatar.card.instance_id.clone();
    let mut effect_source = source_for_unit(
        &ally,
        Seat::North,
        0,
        Some(RealmReference::from_card(&ally.card)),
    );
    effect_source.cells.push(Cell::parse("C4").unwrap());
    game.position.units = vec![enemy, lower, remote, ally, void];
    let all_allies = UnitSet::Query(UnitCohort {
        area: UnitArea::Realm { region: None },
        kind: Some(super::super::UnitKind::Minion),
        controller: ControllerRelation::Allied,
        relation: None,
        exclude_source: false,
    });
    let magic = install(
        &mut game,
        vec![Effect::GiveStealth {
            recipients: all_allies,
        }],
    );
    let frame = game
        .effect_frame(magic, AbilityEntry::Magic, effect_source, None, None, None)
        .unwrap();
    let cases = [
        (all_allies, ids[..4].to_vec()),
        (
            UnitSet::Query(UnitCohort {
                area: UnitArea::Source,
                kind: None,
                controller: ControllerRelation::Any,
                relation: None,
                exclude_source: true,
            }),
            vec![avatar_id, ids[4].clone()],
        ),
        (
            UnitSet::Query(UnitCohort {
                area: UnitArea::Source,
                kind: Some(super::super::UnitKind::Minion),
                controller: ControllerRelation::Enemy,
                relation: None,
                exclude_source: false,
            }),
            vec![ids[4].clone()],
        ),
        (
            UnitSet::Query(UnitCohort {
                area: UnitArea::Realm {
                    region: Some(Region::Surface),
                },
                kind: Some(super::super::UnitKind::Minion),
                controller: ControllerRelation::Any,
                relation: None,
                exclude_source: false,
            }),
            vec![ids[0].clone(), ids[3].clone(), ids[4].clone()],
        ),
    ];
    for (set, mut expected) in cases {
        expected.sort_unstable();
        assert_eq!(
            game.effect_recipients(&frame, set)
                .unwrap()
                .into_iter()
                .map(|r| r.0)
                .collect::<Vec<_>>(),
            expected
        );
    }
    let mut events = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    assert!(game.position.units.iter().all(|u| u.stealthed));
    assert!(
        game.position
            .units
            .iter()
            .find(|u| u.controller == Seat::South)
            .unwrap()
            .warded
    );
    let mut emitted = events
        .iter()
        .filter(|(kind, _)| kind == "minion-stealthed")
        .map(|(_, payload)| payload["instanceId"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let mut expected = ids[..4]
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    assert_eq!(emitted, expected);
    emitted.dedup();
    assert_eq!(emitted.len(), 4);
}

#[test]
fn ordinary_choice_excludes_only_the_sources_original_realm_incarnation() {
    let mut game = fixture_game();
    let source_unit = minion(&game, "south-spell-3", Seat::North, "choice-source", false);
    let ally = minion(&game, "south-spell-3", Seat::North, "choice-ally", false);
    let enemy = minion(&game, "south-spell-3", Seat::South, "choice-enemy", false);
    let source_id = source_unit.card.instance_id.clone();
    let ally_id = ally.card.instance_id.clone();
    game.position.units = vec![source_unit, ally, enemy];
    let target = UnitTarget::Minion {
        instance_id: source_id.clone(),
        seat: Seat::North,
    };
    let source = game.unit_effect_source(&target).unwrap();
    let spec = crate::ability::UnitChoiceSpec {
        kind: Some(super::super::UnitKind::Minion),
        relation: SpatialRelation::Anywhere,
        allied_only: true,
        exclude_source: true,
        optional: false,
    };
    let choices = game.ability_unit_choices(&source, spec);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].instance_id(), &ally_id);
    game.position.units[0].card.enter_realm().unwrap();
    let choices = game.ability_unit_choices(&source, spec);
    assert_eq!(choices.len(), 2);
    assert!(
        choices
            .iter()
            .any(|choice| choice.instance_id() == &source_id)
    );
}

#[test]
fn source_anchor_tracks_the_live_unit_and_rejects_unresolved_departure_geometry() {
    let mut game = fixture_game();
    let source_unit = minion(&game, "south-spell-3", Seat::North, "moving-source", false);
    let source_id = source_unit.card.instance_id.clone();
    game.position.units = vec![source_unit];
    let source = game
        .unit_effect_source(&UnitTarget::Minion {
            instance_id: source_id,
            seat: Seat::North,
        })
        .unwrap();
    let destination = Cell::parse("B2").unwrap();
    game.position.units[0].location = destination;
    assert_eq!(
        game.effect_anchor(&source).unwrap(),
        (Region::Surface, vec![destination])
    );
    game.position.units.clear();
    assert!(matches!(
        game.effect_anchor(&source),
        Err(GameError::UnsupportedMechanic(_))
    ));
}
