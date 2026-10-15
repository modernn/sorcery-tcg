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
use crate::ability::{ArtifactTokenPlacement, TokenDestination};
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
        manifest["cards"]["north-spell-49"] = json!({
            "cardType": "magic",
            "manaCost": 0,
            "effectProgram": {"effects": [{
                "op": "conjure-token", "token": "north-spell-50", "count": 1,
                "destination": "source"
            }]},
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-48"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 3,
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
        manifest["cards"]["north-spell-50"] = json!({
            "cardType": "artifact",
            "grantsBearerPower": 2,
            "manaCost": null,
            "token": true,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["decks"]["north"]["spellbook"]
            .as_array_mut()
            .expect("north spellbook")
            .retain(|card| card != "north-spell-50");
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
            origin: crate::game::DamageOrigin::Other,
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
    let id = card_id(game, "north-spell-49");
    let selection = effects.iter().find_map(|effect| match effect {
        Effect::Damage { recipients, .. }
        | Effect::Untap { recipients }
        | Effect::Grant { recipients, .. }
        | Effect::GiveStealth { recipients }
        | Effect::Disable { recipients, .. } => match recipients {
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
            UnitSet::Query(_) | UnitSet::Chosen | UnitSet::Bound(_) => None,
        },
        Effect::Draw { .. }
        | Effect::DrawCard
        | Effect::BindUnitSet { .. }
        | Effect::ChooseLocation { .. }
        | Effect::ChooseUnit(_)
        | Effect::SummonToken { .. }
        | Effect::ConjureToken { .. } => None,
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

fn install_carried_conjure_tail(game: &mut Game, destination: TokenDestination) -> CardId {
    install(
        game,
        vec![
            Effect::ConjureToken {
                token: "north-spell-50".to_owned(),
                count: 1,
                destination,
                placement: ArtifactTokenPlacement::Carried,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    )
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
    let first_ref = RealmReference::from_card(&first.card);
    let second = minion(&game, "south-spell-2", Seat::North, "deathrite-b", false);
    let second_id = second.card.instance_id.clone();
    let second_ref = RealmReference::from_card(&second.card);
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
    assert_eq!(game.position.units.len(), 3);
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| { unit.card.instance_id == first_id && unit.death_marked })
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| { unit.card.instance_id == second_id && unit.death_marked })
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| { unit.card.instance_id == caster_id && !unit.death_marked })
    );
    let serialized = game.pending_deathrites_value(
        game.position
            .pending_deathrites
            .as_ref()
            .expect("paused effect"),
    );
    assert_eq!(serialized["continuation"]["kind"], "effect");
    assert!(serialized["continuation"].get("boundUnitSets").is_none());
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
                ActionDescriptor::OrderTriggers { source_instance_id, .. }
                    if source_instance_id == &first_id || source_instance_id == &second_id
            )
        })
        .expect("deathrite order");
    let mut branch = game.clone();
    let mut exhausted = game.clone();
    exhausted.position.players[seat_index(Seat::North)]
        .spellbook
        .clear();
    let ignored = exhausted.clone();
    let before_exhausted_position = exhausted.position.clone();
    let before_exhausted_hash = exhausted.state_hash().unwrap();
    let before_continuation = serialized["continuation"].clone();
    assert_eq!(exhausted.position, ignored.position);
    assert_eq!(
        exhausted.state_hash().unwrap(),
        ignored.state_hash().unwrap()
    );
    assert_eq!(
        exhausted.legal_actions().unwrap(),
        ignored.legal_actions().unwrap(),
        "Record and Ignore branches share the same issued order"
    );
    let (terminal_events, terminal_random) = exhausted
        .apply_action_recorded(&order)
        .expect("the first failed Spellbook draw ends the game");
    let ignored_order = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor == order.descriptor
                && action.seat == order.seat
                && action.state_version == order.state_version
        })
        .expect("same issued order in Ignore branch");
    let mut ignored = ignored;
    ignored
        .apply_action(&ignored_order)
        .expect("Ignore branch reaches the same terminal state");
    assert!(terminal_random.is_empty());
    assert_eq!(exhausted.position, ignored.position);
    assert_eq!(
        exhausted.state_hash().unwrap(),
        ignored.state_hash().unwrap()
    );
    assert_ne!(exhausted.position, before_exhausted_position);
    assert_ne!(exhausted.state_hash().unwrap(), before_exhausted_hash);
    assert_eq!(
        terminal_events
            .iter()
            .map(|(kind, _)| kind.as_str())
            .collect::<Vec<_>>(),
        ["trigger-order-committed", "game-ended"]
    );
    assert_eq!(
        terminal_events[1].1,
        json!({ "loser": Seat::North, "reason": "deck_empty", "winner": Seat::South })
    );
    assert!(exhausted.legal_actions().unwrap().is_empty());
    assert_eq!(exhausted.position.phase, Phase::Terminal);
    assert_eq!(
        exhausted.authoritative_state()["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    let pending = exhausted
        .position
        .pending_deathrites
        .as_ref()
        .expect("terminal marked wave retains its owner");
    let terminal_continuation = exhausted.pending_deathrites_value(pending)["continuation"].clone();
    assert_eq!(terminal_continuation, before_continuation);
    assert_eq!(terminal_continuation["kind"], "effect");
    assert_eq!(
        terminal_continuation["magic"]["instanceId"],
        json!(magic_id)
    );
    assert_eq!(
        terminal_continuation["cursor"], 1,
        "the next Draw remains unexecuted in the held Effect frame"
    );
    for reference in [&first_ref, &second_ref] {
        let victim = exhausted
            .position
            .units
            .iter()
            .find(|unit| reference.matches(&unit.card))
            .expect("marked victim remains live");
        assert!(victim.death_marked);
        assert!(
            exhausted.position.players[seat_index(victim.card.owner)]
                .cemetery
                .iter()
                .all(|card| card.instance_id != victim.card.instance_id)
        );
    }
    assert!(
        exhausted
            .position
            .units
            .iter()
            .any(|unit| { unit.card.instance_id == caster_id && !unit.death_marked })
    );
    assert!(
        exhausted.position.players[seat_index(Seat::North)]
            .cemetery
            .iter()
            .all(|card| card.instance_id != magic_id)
    );
    assert!(!terminal_events.iter().any(|(kind, _)| {
        ["spell-drawn", "minion-died", "magic-resolved"].contains(&kind.as_str())
    }));
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
#[expect(
    clippy::too_many_lines,
    reason = "one paused cohort proof tracks departure, movement, reentry, and exact continuation"
)]
fn bound_cohort_survives_deathrite_pause_source_departure_and_incarnation_changes() {
    let mut game = fixture_game();
    let source_unit = minion(&game, "north-spell-48", Seat::North, "bound-source", false);
    let source_id = source_unit.card.instance_id.clone();
    let source_ref = RealmReference::from_card(&source_unit.card);
    let departed = minion(&game, "south-spell-1", Seat::North, "bound-departed", false);
    let departed_id = departed.card.instance_id.clone();
    let mut returned = departed.clone();
    let departed_second = minion(
        &game,
        "south-spell-2",
        Seat::North,
        "bound-departed-second",
        false,
    );
    let survivor = minion(
        &game,
        "north-spell-48",
        Seat::North,
        "bound-survivor",
        false,
    );
    let survivor_id = survivor.card.instance_id.clone();
    game.position.units = vec![source_unit, departed, departed_second, survivor];
    for (cell, label) in [("C3", "bound-origin"), ("A1", "bound-destination")] {
        let location = Cell::parse(cell).unwrap();
        let mut site = card(&game, "south-site-1", Seat::North, label);
        site.enter_realm().unwrap();
        game.position.sites[location.index()] = Some(super::super::SitePosition {
            card: site,
            controller: Seat::North,
            last_flight_turn: None,
            warded: false,
        });
    }
    let mut source = source_for_unit(&game.position.units[0], Seat::North, 0, Some(source_ref));
    source.cells = vec![Cell::parse("C3").unwrap()];
    let magic = install(
        &mut game,
        vec![
            Effect::BindUnitSet {
                id: 7,
                query: UnitCohort {
                    area: UnitArea::Source,
                    kind: Some(super::super::UnitKind::Minion),
                    controller: ControllerRelation::Any,
                    relation: None,
                    exclude_source: false,
                },
            },
            Effect::Damage {
                recipients: UnitSet::Bound(7),
                amount: 1,
            },
            Effect::Disable {
                recipients: UnitSet::Bound(7),
                duration: crate::ability::EffectDuration::UntilYourNextTurn,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let frame = game
        .effect_frame(magic, AbilityEntry::Magic, source, None, None, None)
        .unwrap();
    let mut events = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    assert!(
        game.position.pending_deathrites.is_some(),
        "expected the Deathrite to suspend the bound cohort"
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == source_id)
    );
    let pending = game.pending_deathrites_value(game.position.pending_deathrites.as_ref().unwrap());
    assert_eq!(pending["continuation"]["boundUnitSets"][0]["id"], 7);
    assert_eq!(
        pending["continuation"]["boundUnitSets"][0]["objects"]
            .as_array()
            .unwrap()
            .len(),
        4
    );

    // The source may depart while triggers are ordered. A returned incarnation and a new arrival
    // are absent from the captured set even though both now occupy its original area.
    game.position
        .units
        .retain(|unit| unit.card.instance_id != source_id);
    returned.card.enter_realm().unwrap();
    game.position.units.push(returned.clone());
    let newcomer = minion(
        &game,
        "north-spell-48",
        Seat::North,
        "bound-newcomer",
        false,
    );
    let newcomer_id = newcomer.card.instance_id.clone();
    game.position.units.push(newcomer);
    let survivor = game
        .position
        .units
        .iter_mut()
        .find(|unit| unit.card.instance_id == survivor_id)
        .unwrap();
    survivor.location = Cell::parse("A1").unwrap();

    let order = game
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(&action.descriptor,
            ActionDescriptor::OrderTriggers { source_instance_id, .. }
                if source_instance_id == &departed_id)
        })
        .expect("bound cohort Deathrite order");
    let mut branch = game.clone();
    let branch_order = branch
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| action.descriptor == order.descriptor)
        .expect("same deterministic Deathrite action");
    let (resolved, _) = game.apply_action_recorded(&order).unwrap();
    let (branch_events, _) = branch.apply_action_recorded(&branch_order).unwrap();
    assert_eq!(branch_events, resolved);
    assert_eq!(branch.position, game.position);
    assert_eq!(game.position.pending_deathrites, None);
    let survivor = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == survivor_id)
        .unwrap();
    assert_eq!(survivor.disable_effects.len(), 1);
    assert!(
        game.position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == newcomer_id)
            .unwrap()
            .disable_effects
            .is_empty()
    );
    let returned = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == departed_id)
        .unwrap();
    assert_eq!(returned.card.realm_entry, 2);
    assert!(returned.disable_effects.is_empty());
    assert_eq!(
        resolved
            .iter()
            .filter(|(kind, payload)| kind == "minion-disabled"
                && payload["instanceId"] == json!(survivor_id))
            .count(),
        1
    );
    assert_eq!(
        resolved
            .iter()
            .filter(|(kind, payload)| {
                kind == "spell-drawn" && payload["sourceInstanceId"] == json!(source_id)
            })
            .count(),
        1
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
        events
            .iter()
            .find(|(kind, _)| kind == "ward-broken")
            .map(|(_, payload)| payload.clone()),
        Some(json!({
            "cell": location.cell,
            "instanceId": game.position.sites[location.cell.index()]
                .as_ref()
                .unwrap()
                .card
                .instance_id,
            "seat": "south",
        })),
        "the shared helper keeps the compiled three-field event payload"
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
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "ward-broken")
            .count(),
        1,
        "consuming the Ward protects every dependent operation in this frame"
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
fn carried_conjure_uses_creator_as_owner_and_bearer_controller_as_controller() {
    let mut game = fixture_game();
    let bearer = minion(&game, "south-spell-1", Seat::South, "enemy-bearer", false);
    let bearer_ref = RealmReference::from_card(&bearer.card);
    game.position.units.push(bearer);
    let magic = install_carried_conjure_tail(&mut game, TokenDestination::Chosen);
    let mut frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("creator", Seat::North, 0, None),
            None,
            None,
            None,
        )
        .expect("carried conjure frame");
    frame.chosen = Some(bearer_ref);
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("carried token enters");

    assert_eq!(game.position.artifacts.len(), 1);
    let artifact = &game.position.artifacts[0];
    assert_eq!(artifact.card.owner, Seat::North);
    assert!(artifact.carried_by(
        super::super::UnitKind::Minion,
        Seat::South,
        &game.position.units[0].card.instance_id
    ));
    assert_eq!(
        artifact.bearer().expect("carried bearer").seat(),
        Seat::South
    );
    assert_eq!(
        game.position.units[0].last_picked_up_artifacts_turn, None,
        "entry does not consume the ordinary Pick Up allowance"
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        4,
        "the independent tail still resolves"
    );
}

#[test]
fn carried_conjure_preserves_selected_cell_for_an_oversized_bearer() {
    let mut game = fixture_game();
    let mut bearer = minion(&game, "south-spell-1", Seat::North, "large-bearer", false);
    bearer.occupied_cells =
        Some(["C3", "C4", "D3", "D4"].map(|cell| Cell::parse(cell).expect("occupied cell")));
    let bearer_ref = RealmReference::from_card(&bearer.card);
    game.position.units.push(bearer);
    let magic = install_carried_conjure_tail(&mut game, TokenDestination::Chosen);
    let mut frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            source("large-creator", Seat::North, 0, None),
            None,
            None,
            None,
        )
        .expect("oversized carried frame");
    frame.chosen = Some(bearer_ref);
    frame.token_location = Some(super::ResolvedTokenLocation {
        location: Some(Location {
            cell: Cell::parse("D4").expect("selected cell"),
            region: Region::Surface,
        }),
    });
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .expect("oversized token enters");

    let artifact = &game.position.artifacts[0];
    let super::super::ArtifactPlacement::Carried { cell, .. } = &artifact.placement else {
        panic!("token should enter carried");
    };
    assert_eq!(*cell, Some(Cell::parse("D4").expect("selected cell")));
    assert_eq!(
        game.artifact_location(artifact)
            .expect("artifact location")
            .cell,
        Cell::parse("D4").expect("selected cell")
    );
}

#[test]
fn stale_chosen_bearer_skips_carried_creation_and_continues_tail() {
    for destination in [TokenDestination::Chosen, TokenDestination::Source] {
        let mut game = fixture_game();
        let bearer = minion(&game, "south-spell-1", Seat::South, "stale-bearer", false);
        let stale_ref = RealmReference::from_card(&bearer.card);
        let mut reentered = bearer;
        reentered.card.enter_realm().expect("new incarnation");
        game.position.units.push(reentered);
        let magic = install_carried_conjure_tail(&mut game, destination);
        let mut frame = game
            .effect_frame(
                magic,
                AbilityEntry::Magic,
                source("stale-target", Seat::North, 0, None),
                None,
                None,
                None,
            )
            .expect("stale carried frame");
        if destination == TokenDestination::Source {
            // The source departed after the program started, retaining last-known geometry.
            frame.started = true;
            frame.source.realm = Some(stale_ref);
            frame.source.cells = ["C3", "C4", "D3", "D4"]
                .map(|c| Cell::parse(c).unwrap())
                .to_vec();
        } else {
            frame.chosen = Some(stale_ref);
        }
        game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
            .expect("stale carried entry is a skipped operation");

        assert!(game.position.artifacts.is_empty());
        assert_eq!(
            game.position.players[seat_index(Seat::North)]
                .hand_spellbook
                .len(),
            4,
            "the independent tail still resolves after stale selection"
        );
    }
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
fn disable_query_and_chosen_cohorts_apply_ward_and_stack_once() {
    let mut game = fixture_game();
    let source_unit = minion(&game, "south-spell-3", Seat::North, "disable-source", false);
    let source_id = source_unit.card.instance_id.clone();
    let mut enemy = minion(&game, "south-spell-3", Seat::South, "disable-enemy", false);
    enemy.warded = true;
    game.position.units = vec![source_unit, enemy];
    let mut site = card(&game, "south-site-1", Seat::North, "disable-site");
    site.enter_realm().unwrap();
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    let source = source_for_unit(
        &game.position.units[0],
        Seat::North,
        0,
        Some(RealmReference::from_card(&game.position.units[0].card)),
    );
    let enemy_id = game.position.units[1].card.instance_id.clone();
    let query = UnitSet::Query(UnitCohort {
        area: UnitArea::Realm { region: None },
        kind: Some(super::super::UnitKind::Minion),
        controller: ControllerRelation::Enemy,
        relation: None,
        exclude_source: false,
    });
    let query_magic = install(
        &mut game,
        vec![Effect::Disable {
            recipients: query,
            duration: crate::ability::EffectDuration::UntilYourNextTurn,
        }],
    );
    let query_frame = game
        .effect_frame(
            query_magic,
            AbilityEntry::Magic,
            source.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    let mut events = Vec::new();
    game.run_effect_frame(query_frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    let enemy = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == enemy_id)
        .unwrap();
    assert!(!enemy.warded);
    assert_eq!(enemy.disable_effects.len(), 1);

    let chosen_magic = install(
        &mut game,
        vec![
            Effect::ChooseUnit(crate::ability::UnitChoiceSpec {
                kind: Some(super::super::UnitKind::Minion),
                relation: SpatialRelation::Anywhere,
                exclude_source: false,
                allied_only: false,
                optional: false,
            }),
            Effect::Disable {
                recipients: UnitSet::Chosen,
                duration: crate::ability::EffectDuration::UntilYourNextTurn,
            },
        ],
    );
    let chosen_source_id = source.instance_id.clone();
    let chosen_frame = game
        .effect_frame(chosen_magic, AbilityEntry::Magic, source, None, None, None)
        .unwrap();
    game.run_effect_frame(chosen_frame, &mut OutcomeLog::Ignore)
        .unwrap();
    game.apply_ability_choice_action(
        Seat::North,
        &chosen_source_id,
        Some(&UnitTarget::Minion {
            instance_id: enemy_id.clone(),
            seat: Seat::South,
        }),
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    let enemy = game
        .position
        .units
        .iter()
        .find(|unit| unit.card.instance_id == enemy_id)
        .unwrap();
    assert_eq!(enemy.disable_effects.len(), 2);
    assert_eq!(game.position.pending_ability_choice, None);
    assert_eq!(source_id, game.position.units[0].card.instance_id);
}

#[test]
fn disable_settles_before_an_ordered_draw_tail() {
    let mut game = fixture_game();
    let source_unit = minion(
        &game,
        "south-spell-3",
        Seat::North,
        "disable-tail-source",
        false,
    );
    let mut target = minion(
        &game,
        "south-spell-3",
        Seat::South,
        "disable-tail-target",
        false,
    );
    target.stealthed = true;
    game.position.units = vec![source_unit, target];
    let mut site = card(&game, "south-site-1", Seat::North, "disable-tail-site");
    site.enter_realm().unwrap();
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    let source = source_for_unit(
        &game.position.units[0],
        Seat::North,
        0,
        Some(RealmReference::from_card(&game.position.units[0].card)),
    );
    let magic = install(
        &mut game,
        vec![
            Effect::Disable {
                recipients: UnitSet::Query(UnitCohort {
                    area: UnitArea::Realm { region: None },
                    kind: Some(super::super::UnitKind::Minion),
                    controller: ControllerRelation::Enemy,
                    relation: None,
                    exclude_source: false,
                }),
                duration: crate::ability::EffectDuration::UntilYourNextTurn,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let mut events = Vec::new();
    let before = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let frame = game
        .effect_frame(magic, AbilityEntry::Magic, source, None, None, None)
        .unwrap();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .unwrap();
    let kinds = events
        .iter()
        .map(|(kind, _)| kind.as_str())
        .collect::<Vec<_>>();
    assert!(
        kinds.iter().position(|kind| *kind == "minion-disabled")
            < kinds.iter().position(|kind| *kind == "spell-drawn")
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before + 1
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one interrupted disable proves both settlement stages resume once"
)]
fn disable_settlement_survives_region_and_static_power_interruptions() {
    let mut game = fixture_game();
    let site_id = card_id(&game, "south-site-1");
    let provider_id = card_id(&game, "south-spell-1");
    let ally_id = card_id(&game, "south-spell-2");
    let deathrite_id = card_id(&game, "south-spell-3");
    let second_deathrite_id = card_id(&game, "south-spell-4");
    let cascade_id = card_id(&game, "south-spell-5");
    {
        let rules = Arc::get_mut(&mut game.rules).expect("fixture rules are unique");
        let super::super::CardFacts::Site(facts) = &mut rules.cards[usize::from(site_id.0)].facts
        else {
            unreachable!()
        };
        facts.elements = crate::facts::ElementSet::only(crate::facts::Element::Water);
        for (
            id,
            submerge,
            deathrite_draw_site,
            deathrite_draw_spells,
            deathrite_heal,
            nearby_bonus,
        ) in [
            (provider_id, true, false, false, None, true),
            (ally_id, true, false, false, None, true),
            (deathrite_id, false, true, false, None, false),
            (second_deathrite_id, false, false, false, Some(1), false),
            (cascade_id, true, false, false, None, false),
        ] {
            let super::super::CardFacts::Minion(facts) = &mut rules.cards[usize::from(id.0)].facts
            else {
                unreachable!()
            };
            facts.submerge = submerge;
            facts.deathrite_draw_site = deathrite_draw_site;
            facts.deathrite_draw_spells = deathrite_draw_spells;
            facts.deathrite_heal = deathrite_heal;
            facts.other_nearby_allies_power_bonus = nearby_bonus;
            if id == cascade_id {
                facts.defense = 1;
            }
        }
    }

    let mut site = card(
        &game,
        "south-site-1",
        Seat::North,
        "disable-settlement-site",
    );
    site.enter_realm().expect("site realm entry");
    game.position.sites[Cell::parse("C3").unwrap().index()] = Some(super::super::SitePosition {
        card: site,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    let mut provider = minion(
        &game,
        "south-spell-1",
        Seat::North,
        "disable-settlement-provider",
        false,
    );
    provider.region = Region::Underwater;
    let provider_instance_id = provider.card.instance_id.clone();
    let mut ally = minion(
        &game,
        "south-spell-2",
        Seat::North,
        "disable-settlement-ally",
        false,
    );
    ally.region = Region::Underwater;
    ally.damage = 0;
    let ally_instance_id = ally.card.instance_id.clone();
    let mut cascade = minion(
        &game,
        "south-spell-5",
        Seat::North,
        "disable-settlement-cascade",
        false,
    );
    cascade.region = Region::Underwater;
    cascade.damage = 1;
    let cascade_instance_id = cascade.card.instance_id.clone();
    let mut deathrite = minion(
        &game,
        "south-spell-3",
        Seat::North,
        "disable-settlement-deathrite",
        false,
    );
    deathrite.region = Region::Underwater;
    let deathrite_instance_id = deathrite.card.instance_id.clone();
    let mut second_deathrite = minion(
        &game,
        "south-spell-4",
        Seat::North,
        "disable-settlement-second-deathrite",
        false,
    );
    second_deathrite.region = Region::Underwater;
    game.position.units = vec![provider, ally, cascade, deathrite];
    game.position.units.push(second_deathrite);
    assert_eq!(
        game.minion_current_stats(&game.position.units[1])
            .unwrap()
            .1,
        2
    );

    let magic = install(
        &mut game,
        vec![
            Effect::Disable {
                recipients: UnitSet::Target,
                duration: crate::ability::EffectDuration::UntilYourNextTurn,
            },
            Effect::Draw {
                zone: DeckZone::Spellbook,
                count: 1,
            },
        ],
    );
    let target = UnitTarget::Minion {
        instance_id: provider_instance_id.clone(),
        seat: Seat::North,
    };
    let before_draw = game.position.players[seat_index(Seat::North)]
        .hand_spellbook
        .len();
    let mut effect_source = source("disable-settlement-source", Seat::North, 0, None);
    effect_source.region = Region::Underwater;
    let effect_source_id = effect_source.instance_id.clone();
    let frame = game
        .effect_frame(
            magic,
            AbilityEntry::Magic,
            effect_source,
            Some(&target),
            None,
            None,
        )
        .expect("disable frame");
    let mut events = Vec::new();
    game.run_effect_frame(frame, &mut OutcomeLog::Record(&mut events))
        .expect("disable pauses for region deathrite");
    assert!(game.position.pending_deathrites.is_some());
    let pending = game.position.pending_deathrites.as_ref().unwrap();
    assert_eq!(
        game.pending_deathrites_value(pending)["continuation"]["pendingSettlement"],
        json!("region")
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| { unit.card.instance_id == provider_instance_id && unit.death_marked })
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == ally_instance_id)
    );
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == cascade_instance_id)
    );
    game.position
        .units
        .iter_mut()
        .find(|unit| unit.card.instance_id == ally_instance_id)
        .expect("ally survived region settlement")
        .damage = 1;

    let order = game
        .legal_actions()
        .expect("deathrite order actions")
        .into_iter()
        .find(|action| {
            matches!(
                &action.descriptor,
                ActionDescriptor::OrderTriggers { source_instance_id, .. }
                    if source_instance_id == &deathrite_instance_id
            )
        })
        .expect("deathrite order");
    let mut replay = game.clone();
    let (replay_events, _) = replay
        .apply_action_recorded(&order)
        .expect("resume cloned checkpoint");
    let (resumed_events, _) = game
        .apply_action_recorded(&order)
        .expect("resume settlement");
    assert_eq!(replay_events, resumed_events);
    assert_eq!(replay.state_hash().unwrap(), game.state_hash().unwrap());
    assert!(
        !game
            .position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == ally_instance_id)
    );
    assert!(
        !game
            .position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == cascade_instance_id)
    );
    assert_eq!(
        game.position.players[seat_index(Seat::North)]
            .hand_spellbook
            .len(),
        before_draw + 1
    );
    assert_eq!(
        resumed_events
            .iter()
            .filter(|(kind, payload)| {
                kind == "site-drawn" && payload["sourceInstanceId"] == json!(deathrite_instance_id)
            })
            .count(),
        1
    );
    assert_eq!(
        resumed_events
            .iter()
            .filter(|(kind, payload)| {
                kind == "spell-drawn" && payload["sourceInstanceId"] == json!(effect_source_id)
            })
            .count(),
        1
    );
    assert_eq!(
        resumed_events
            .iter()
            .filter(|(kind, payload)| {
                kind == "minion-died" && payload["instanceId"] == json!(ally_instance_id)
            })
            .count(),
        1
    );
    assert!(events.iter().any(|(kind, _)| kind == "minion-disabled"));
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

// These complete synthetic fixtures prove shared rules, not actual card admission.
mod site_token_proofs {
    use super::super::super::{CardFacts, PendingField, PendingGenesisToken, SitePosition};
    use super::*;
    use crate::ability::{SiteDestinationArea, SiteDestinationQuery};

    fn fixture(seat: Seat) -> Game {
        fixture_with_facts(seat, None)
    }

    fn fixture_with_facts(seat: Seat, extra: Option<&serde_json::Value>) -> Game {
        let manifest = selfplay_manifest_with(141_001, |manifest| {
            let mut facts: serde_json::Value = serde_json::from_str(r#"{"syn:land":{"cardType":"site","elements":["earth"]},"syn:water":{"cardType":"site","elements":["water"]},"syn:bar-land":{"cardType":"site","elements":["earth"],"preventsUnitsWithPowerAtLeastFromEntering":3},"syn:bar-water":{"cardType":"site","elements":["water"],"preventsUnitsWithPowerAtLeastFromEntering":3},"syn:bonus":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"otherNearbyAlliesPowerBonus":1},"syn:power2":{"cardType":"minion","attack":2,"defense":2,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"burrowing":true,"submerge":true},"syn:power3":{"cardType":"minion","attack":3,"defense":3,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"burrowing":true,"submerge":true},"syn:plain-token":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true},"syn:single-site":{"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"selection":null,"effects":[{"op":"summon-token","token":"syn:plain-token","count":1,"destination":{"choose-site":{"area":"realm","controller":"allied"}}}]}},"syn:parent":{"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"selection":null,"effects":[{"op":"summon-token","token":"syn:entry-token","count":1,"destination":{"each-site":{"area":"near-controller-avatar","controller":"any"}}},{"op":"draw","zone":"spellbook","count":1}]}},"syn:entry-token":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"entersCarrying":["syn:gear"],"genesisProgram":{"effects":[{"op":"choose-unit","relation":"anywhere","alliedOnly":true,"optional":true},{"op":"draw","zone":"spellbook","count":1}]}},"syn:gear":{"cardType":"artifact","manaCost":null,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"bearerUnitStrike":{"damageBonus":1}},"syn:death-parent":{"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"selection":null,"effects":[{"op":"summon-token","token":"syn:death-token","count":1,"destination":{"each-site":{"area":"near-controller-avatar","controller":"any"}}},{"op":"draw","zone":"spellbook","count":1}]}},"syn:death-token":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"deathriteDrawSite":true},"syn:entry-stealth":{"cardType":"site","elements":["earth"],"siteEntryEffect":"grantStealthToEnteringMinion"},"syn:mixed-identity":{"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"selection":null,"effects":[{"op":"summon-token","token":"syn:plain-token","count":1,"destination":{"each-site":{"area":"near-controller-avatar","controller":"any"}}},{"op":"summon-token","token":"syn:plain-token","count":1,"destination":"source"}]}},"syn:genesis-site-summoner":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"genesisProgram":{"effects":[{"op":"summon-token","token":"syn:plain-token","count":1,"destination":{"each-site":{"area":"near-controller-avatar","controller":"any"}}}]}},"syn:paid-bar":{"cardType":"site","elements":["earth"],"preventsUnitsWithPowerAtLeastFromEntering":3,"genesisPayOneManaToSummonToken":"syn:power2"},"syn:paid-bar-ordinary":{"cardType":"site","elements":["earth"],"ordinary":true,"preventsUnitsWithPowerAtLeastFromEntering":3,"genesisPayOneManaToSummonToken":"syn:power2"},"syn:paid-clear":{"cardType":"site","elements":["earth"],"genesisPayOneManaToSummonToken":"syn:power2"},"syn:fate-contrast":{"cardType":"aura","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities":true},"syn:legacy-bar-magic":{"cardType":"magic","manaCost":2,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"summonTokenToEachControlledSiteBorderingEnemySite":"syn:power2"},"syn:power3-spell":{"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"selection":null,"effects":[{"op":"summon-token","token":"syn:power3","count":1,"destination":{"each-site":{"area":"near-controller-avatar","controller":"any"}}}]}},"syn:square-power2":{"cardType":"minion","attack":2,"defense":2,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"token":true,"occupiesSquareArea":2}}"#).unwrap();
            if let Some(extra) = extra {
                facts
                    .as_object_mut()
                    .unwrap()
                    .extend(extra.as_object().unwrap().clone());
            }
            manifest["cards"]["north-spell-49"] = json!({"cardType":"magic","manaCost":0,
                "thresholds":{"air":0,"earth":0,"fire":0,"water":0},
                "effectProgram":{"effects":[{"op":"summon-token","token":"syn:square-power2","count":1,"destination":"source"}]}});
            for (id, facts) in facts.as_object().unwrap() {
                manifest["cards"][id] = facts.clone();
                if facts["token"] != true {
                    let zone = if facts["cardType"] == "site" {
                        "atlas"
                    } else {
                        "spellbook"
                    };
                    manifest["decks"]["north"][zone]
                        .as_array_mut()
                        .unwrap()
                        .push(json!(id));
                }
            }
        });
        let mut game = Game::from_manifest_json(&manifest).unwrap();
        game.position.sites.fill(None);
        game.position.rubble.fill(None);
        game.position.units.clear();
        game.position.phase = Phase::Main;
        game.position.active_seat = seat;
        game.position.decision_seat = seat;
        game.position.turn_number = 10;
        game.position.state_version = 100;
        for player in &mut game.position.players {
            player.mana = 12;
            player.domain_established = true;
        }
        game.position.players[seat_index(seat)].avatar.location = cell(seat, "C2");
        game.position.players[seat_index(super::super::super::other_seat(seat))]
            .avatar
            .location = cell(seat, "E1");
        for (at, id, allied, warded) in [
            ("C2", "land", true, false),
            ("B2", "land", true, false),
            ("B1", "water", true, false),
            ("D2", "water", true, false),
            ("D3", "land", false, false),
            ("B3", "land", false, true),
            ("E4", "land", true, false),
            ("A4", "water", true, false),
            ("E1", "land", false, false),
        ] {
            put_site(&mut game, seat, at, id, allied, warded);
        }
        game.position.rubble[cell(seat, "C1").index()] =
            Some(identity_hash(&json!({"rubble":"C1","seat":seat})).unwrap());
        game
    }

    fn cell(seat: Seat, at: &str) -> Cell {
        let bytes = at.as_bytes();
        let at = if seat == Seat::South {
            format!(
                "{}{}",
                char::from(b'E' - (bytes[0] - b'A')),
                char::from(b'5' - (bytes[1] - b'0'))
            )
        } else {
            at.to_owned()
        };
        Cell::parse(&at).unwrap()
    }

    fn put_site(game: &mut Game, seat: Seat, at: &str, id: &str, allied: bool, warded: bool) {
        let at = cell(seat, at);
        let owner = if allied {
            seat
        } else {
            super::super::super::other_seat(seat)
        };
        let mut card = card(game, &format!("syn:{id}"), owner, &format!("site-{at:?}"));
        card.enter_realm().unwrap();
        game.position.rubble[at.index()] = None;
        game.position.sites[at.index()] = Some(SitePosition {
            card,
            controller: owner,
            warded,
            last_flight_turn: None,
        });
    }

    fn query(each: bool) -> TokenDestination {
        let q = SiteDestinationQuery {
            area: if each {
                SiteDestinationArea::NearControllerAvatar
            } else {
                SiteDestinationArea::Realm
            },
            controller: if each {
                ControllerRelation::Any
            } else {
                ControllerRelation::Allied
            },
        };
        if each {
            TokenDestination::EachSite(q)
        } else {
            TokenDestination::ChooseSite(q)
        }
    }

    fn frame(game: &Game, seat: Seat, id: &str) -> super::super::EffectFrame {
        game.effect_frame(
            card_id(game, id),
            AbilityEntry::Magic,
            source("site-token", seat, 0, None),
            None,
            None,
            Some(card(game, id, seat, "held-magic")),
        )
        .unwrap()
    }

    fn choose_all(game: &mut Game, outcomes: &mut OutcomeLog<'_>) {
        while let Some(pending) = &game.position.pending_ability_choice {
            let source = pending.frame.source.instance_id.clone();
            let seat = pending.frame.source.controller;
            let location = game
                .site_token_location_choices(&pending.frame)
                .unwrap()
                .into_iter()
                .find(|location| location.region == Region::Surface)
                .unwrap();
            game.apply_ability_location_choice_action(seat, &source, location, outcomes)
                .unwrap();
        }
    }

    #[test]
    fn non_target_geometry_layers_ward_cohort_identity_and_both_seats() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            let destinations = game.site_destination_cells(
                match query(true) {
                    TokenDestination::EachSite(q) => q,
                    _ => unreachable!(),
                },
                seat,
            );
            let expected: Vec<_> = ["C2", "B2", "B1", "D2", "D3", "B3", "C1"]
                .map(|at| cell(seat, at))
                .into();
            assert_eq!(destinations.len(), expected.len());
            assert!(expected.iter().all(|cell| destinations.contains(cell)));
            let id = install(
                &mut game,
                vec![Effect::SummonToken {
                    token: "syn:plain-token".into(),
                    count: 1,
                    destination: query(true),
                }],
            );
            let mut f = frame(&game, seat, "north-spell-49");
            f.card_id = id;
            // Caster geometry does not restrict this controller-Avatar site query.
            f.source.region = Region::Void;
            f.source.cells = vec![cell(seat, "A4")];
            let mut events = Vec::new();
            let mut outcomes = OutcomeLog::Record(&mut events);
            game.run_effect_frame(f, &mut outcomes).unwrap();
            assert!(game.position.units.is_empty());
            let pending = &game.position.pending_ability_choice.as_ref().unwrap().frame;
            assert_eq!(
                game.effect_frame_value(pending)["siteTokens"]["sites"]
                    .as_array()
                    .unwrap()
                    .len(),
                7
            );
            let cloned = game.clone();
            assert_eq!(cloned.state_hash().unwrap(), game.state_hash().unwrap());
            choose_all(&mut game, &mut outcomes);
            assert_eq!(game.position.units.len(), 7);
            let ids: std::collections::BTreeSet<_> = game
                .position
                .units
                .iter()
                .map(|u| &u.card.instance_id)
                .collect();
            assert_eq!(ids.len(), 7);
            assert!(game.position.units.iter().all(|u| u.controller == seat
                && u.card.owner == seat
                && u.summoning_sickness
                && !u.tapped));
            assert!(
                game.position.sites[cell(seat, "B3").index()]
                    .as_ref()
                    .unwrap()
                    .warded
            );
        }
    }

    #[test]
    fn chosen_site_count_hold_empty_completion_and_stale_incarnations() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            let id = install(
                &mut game,
                vec![Effect::SummonToken {
                    token: "syn:plain-token".into(),
                    count: 4,
                    destination: query(false),
                }],
            );
            let mut f = frame(&game, seat, "north-spell-49");
            f.card_id = id;
            let before = game.authoritative_state();
            let mut events = Vec::new();
            assert!(matches!(
                game.run_effect_frame(f, &mut OutcomeLog::Record(&mut events)),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert!(game.position.units.is_empty());
            assert!(game.position.pending_ability_choice.is_none());
            assert_eq!(before, game.authoritative_state());
            install(
                &mut game,
                vec![Effect::SummonToken {
                    token: "syn:plain-token".into(),
                    count: 1,
                    destination: query(false),
                }],
            );
            let f = frame(&game, seat, "north-spell-49");
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            let pending = game.position.pending_ability_choice.as_ref().unwrap();
            let choices = game.site_token_location_choices(&pending.frame).unwrap();
            assert!(choices.contains(&Location {
                cell: cell(seat, "E4"),
                region: Region::Underground
            }));
            assert!(
                !choices
                    .iter()
                    .any(|l| l.cell == cell(seat, "C1") || l.cell == cell(seat, "B3"))
            );
            let c = cell(seat, "B2");
            let captured = game.capture_site_destination(c).unwrap().unwrap();
            game.position.sites[c.index()]
                .as_mut()
                .unwrap()
                .card
                .enter_realm()
                .unwrap();
            assert!(game.site_destination_cell(&captured).is_none());
            let r = cell(seat, "C1");
            let captured = game.capture_site_destination(r).unwrap().unwrap();
            game.position.rubble[r.index()] =
                Some(identity_hash(&json!({"new":"rubble"})).unwrap());
            assert!(game.site_destination_cell(&captured).is_none());
            game.position.sites.fill(None);
            game.position.rubble.fill(None);
            game.resume_empty_ability_choice(&mut OutcomeLog::Ignore)
                .unwrap();
            assert!(game.position.pending_ability_choice.is_none());
            assert!(game.position.units.is_empty());
        }
    }

    #[test]
    fn all_layer_entry_and_c1_preflight_are_distinct_from_survival() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            for (at, site, region) in [
                ("B2", "bar-land", Region::Underground),
                ("D2", "bar-water", Region::Underwater),
            ] {
                put_site(&mut game, seat, at, site, true, false);
                let location = Location {
                    cell: cell(seat, at),
                    region,
                };
                assert!(
                    game.token_may_enter_location(seat, "syn:power2", location)
                        .unwrap()
                );
                assert!(
                    !game
                        .token_may_enter_location(seat, "syn:power3", location)
                        .unwrap()
                );
                assert!(matches!(
                    game.create_token_unit(
                        seat,
                        "syn:power3",
                        &source("ctor", seat, 0, None).instance_id,
                        location,
                        0,
                        100
                    ),
                    Err(GameError::IllegalAction)
                ));
            }
            let mut bonus = minion(&game, "syn:bonus", seat, "bonus", false);
            bonus.location = cell(seat, "B2");
            game.position.units.push(bonus);
            assert!(matches!(
                game.token_may_enter_cell(seat, "syn:power2", cell(seat, "B2")),
                Err(GameError::UnsupportedMechanic(_))
            ));
            install(
                &mut game,
                vec![Effect::SummonToken {
                    token: "syn:power2".into(),
                    count: 1,
                    destination: query(true),
                }],
            );
            let f = frame(&game, seat, "north-spell-49");
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            loop {
                let pending = game.position.pending_ability_choice.as_ref().unwrap();
                let source = pending.frame.source.instance_id.clone();
                let location = game
                    .site_token_location_choices(&pending.frame)
                    .unwrap()
                    .into_iter()
                    .find(|l| l.region == Region::Surface)
                    .unwrap();
                match game.apply_ability_location_choice_action(
                    seat,
                    &source,
                    location,
                    &mut OutcomeLog::Ignore,
                ) {
                    Ok(()) => {}
                    Err(GameError::UnsupportedMechanic(_)) => break,
                    other => panic!("unexpected {other:?}"),
                }
            }
            assert_eq!(
                game.position.units.len(),
                1,
                "no successful token prefix before C1"
            );
        }
    }

    #[test]
    fn upstream_incoming_live_footprint_and_deferred_guards() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            let at = cell(seat, "B2");
            let mut bonus = minion(&game, "syn:bonus", seat, "upstream-bonus", false);
            bonus.location = at;
            game.position.units.push(bonus);
            let CardFacts::Site(bar) =
                &game.rules.cards[usize::from(card_id(&game, "syn:paid-bar").0)].facts
            else {
                panic!()
            };
            let CardFacts::Site(clear) =
                &game.rules.cards[usize::from(card_id(&game, "syn:paid-clear").0)].facts
            else {
                panic!()
            };
            assert!(matches!(
                game.token_may_enter_played_site(seat, "syn:power2", at, bar),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert!(
                game.token_may_enter_played_site(seat, "syn:power2", at, clear)
                    .unwrap()
            );
            put_site(&mut game, seat, "B2", "paid-bar", true, false);
            let before = game.authoritative_state();
            assert!(matches!(
                game.magic_token_summon_cells(seat, "syn:power2"),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            let source = game.position.sites[at.index()]
                .as_ref()
                .unwrap()
                .card
                .instance_id
                .clone();
            game.position.pending_genesis_token = PendingField::Pending(PendingGenesisToken {
                cell: at,
                seat,
                source_instance_id: source,
            });
            game.position.phase = Phase::Genesis;
            let before = game.authoritative_state();
            let mut actions = Vec::new();
            assert!(matches!(
                game.append_pending_genesis_token_actions(
                    &mut actions,
                    match &game.position.pending_genesis_token {
                        PendingField::Pending(pending) => pending,
                        _ => panic!(),
                    }
                ),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            game.position.players[seat_index(seat)].mana = 0;
            actions.clear();
            game.append_pending_genesis_token_actions(
                &mut actions,
                match &game.position.pending_genesis_token {
                    PendingField::Pending(pending) => pending,
                    _ => panic!(),
                },
            )
            .unwrap();
            assert_eq!(actions.len(), 1);
        }
    }
    #[test]
    fn parent_equipment_genesis_record_silent_and_captured_custody() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            put_site(&mut game, seat, "C2", "entry-stealth", true, false);
            let mut f = frame(&game, seat, "syn:parent");
            let owner = super::super::super::other_seat(seat);
            f.source.owner = owner;
            f.magic.as_mut().unwrap().owner = owner;
            let magic_id = f.magic.as_ref().unwrap().instance_id.clone();
            let before_spells = game.position.players[seat_index(seat)].spellbook.len();
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            let mut silent = game.clone();
            let mut recorded_again = game.clone();
            let mut all_events = Vec::new();
            for _ in 0..100 {
                if game.position.phase == Phase::Main {
                    break;
                }
                let actions = game.legal_actions().unwrap();
                assert_eq!(actions, silent.legal_actions().unwrap());
                let action = actions
                    .iter()
                    .find(|a| {
                        matches!(
                            a.descriptor,
                            ActionDescriptor::ChooseAbilityLocation {
                                location: Location {
                                    region: Region::Surface,
                                    ..
                                },
                                ..
                            }
                        )
                    })
                    .or_else(|| {
                        actions.iter().find(|a| {
                            matches!(
                                a.descriptor,
                                ActionDescriptor::ChooseAbility { target: None, .. }
                            )
                        })
                    })
                    .unwrap_or_else(|| actions.first().expect("continuation action"));
                let (events, random) = game.apply_action_recorded(action).unwrap();
                let (other, other_random) = recorded_again.apply_action_recorded(action).unwrap();
                silent.apply_action(action).unwrap();
                assert_eq!(events, other);
                assert_eq!(random, other_random);
                assert_eq!(game.authoritative_state(), silent.authoritative_state());
                assert_eq!(game.state_hash().unwrap(), silent.state_hash().unwrap());
                all_events.extend(events);
            }
            assert_eq!(game.position.phase, Phase::Main);
            assert_eq!(game.position.units.len(), 7);
            assert_eq!(game.position.artifacts.len(), 7);
            assert!(
                game.position
                    .units
                    .iter()
                    .find(|unit| unit.location == cell(seat, "C2"))
                    .unwrap()
                    .stealthed
            );
            assert_eq!(
                before_spells - game.position.players[seat_index(seat)].spellbook.len(),
                8,
                "seven Genesis draws plus exactly one parent tail"
            );
            assert_eq!(
                game.position.players[seat_index(owner)]
                    .cemetery
                    .iter()
                    .filter(|c| c.instance_id == magic_id)
                    .count(),
                1
            );
            let last_entry = all_events
                .iter()
                .rposition(|(kind, _)| kind == "minion-summoned")
                .unwrap();
            let first_draw = all_events
                .iter()
                .position(|(kind, _)| kind == "spell-drawn")
                .unwrap();
            assert!(
                last_entry < first_draw,
                "whole cohort enters before Genesis"
            );
        }
    }

    #[test]
    fn source_departure_is_explicit_and_started_choice_skips_replacement() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            install(
                &mut game,
                vec![Effect::SummonToken {
                    token: "syn:plain-token".into(),
                    count: 1,
                    destination: query(true),
                }],
            );
            let mut f = frame(&game, seat, "north-spell-49");
            let actor = minion(&game, "syn:bonus", seat, "departed-actor", false);
            f.source.actor = Some(RealmReference::from_card(&actor.card));
            let before = game.authoritative_state();
            assert!(matches!(
                game.run_effect_frame(f, &mut OutcomeLog::Ignore),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            let f = frame(&game, seat, "north-spell-49");
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            let location = game
                .site_token_location_choices(
                    &game.position.pending_ability_choice.as_ref().unwrap().frame,
                )
                .unwrap()[0];
            if let Some(site) = game.position.sites[location.cell.index()].as_mut() {
                site.card.enter_realm().unwrap();
            } else {
                game.position.rubble[location.cell.index()] =
                    Some(identity_hash(&json!({"replacement":true,"seat":seat})).unwrap());
            }
            game.resume_empty_ability_choice(&mut OutcomeLog::Ignore)
                .unwrap();
            choose_all(&mut game, &mut OutcomeLog::Ignore);
            assert_eq!(
                game.position.units.len(),
                6,
                "no retarget to replacement incarnation"
            );
        }
    }

    #[test]
    fn upstream_legacy_incoming_payment_and_decline_are_atomic() {
        use crate::action::GenesisTokenChoice;
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            game.position.players[seat_index(seat)].hand_spellbook =
                vec![card(&game, "syn:legacy-bar-magic", seat, "legacy-hand")];
            let cast=game.legal_actions().unwrap().into_iter().find(|a| matches!(&a.descriptor,ActionDescriptor::CastMagic {card_id,..} if card_id=="syn:legacy-bar-magic")).unwrap();
            put_site(&mut game, seat, "B2", "paid-bar", true, false);
            let mut bonus = minion(&game, "syn:bonus", seat, "legacy-bonus", false);
            bonus.location = cell(seat, "B2");
            game.position.units.push(bonus);
            let before = game.authoritative_state();
            let mut events = Vec::new();
            assert!(matches!(
                game.apply_cast_magic_action(
                    &cast,
                    &mut OutcomeLog::Record(&mut events),
                    None,
                    None
                ),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            assert!(events.is_empty());
            // Paid site issuance and validation use incoming facts even over void.
            let mut game = fixture(seat);
            let at = cell(seat, "C3");
            let mut atlas_card = card(&game, "syn:paid-bar", seat, "incoming-hand");
            atlas_card.source = CardSource::Atlas;
            game.position.players[seat_index(seat)].hand_atlas = vec![atlas_card];
            let mut actions = Vec::new();
            game.append_play_site_actions(&mut actions, seat, None)
                .unwrap();
            let pay=actions.iter().find(|a|matches!(a.descriptor,ActionDescriptor::PlaySite {cell,genesis_token_choice:Some(GenesisTokenChoice::PayOneMana),..} if cell==at)).unwrap().clone();
            let decline=actions.iter().find(|a|matches!(a.descriptor,ActionDescriptor::PlaySite {cell,genesis_token_choice:Some(GenesisTokenChoice::Decline),..} if cell==at)).unwrap().clone();
            let mut bonus = minion(&game, "syn:bonus", seat, "incoming-bonus", false);
            bonus.location = cell(seat, "B2");
            game.position.units.push(bonus);
            let before = game.authoritative_state();
            actions.clear();
            assert!(matches!(
                game.append_play_site_actions(&mut actions, seat, None),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert!(matches!(
                game.apply_play_site_action(&pay, &mut OutcomeLog::Record(&mut events)),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            assert!(events.is_empty());
            game.apply_play_site_action(&decline, &mut OutcomeLog::Ignore)
                .unwrap();
            assert_eq!(
                game.position.units.len(),
                1,
                "decline exercises no token entry"
            );
            // Deferred revalidation preserves pending work, payment and events.
            let site = game.position.sites[at.index()].as_ref().unwrap();
            game.position.pending_genesis_token = PendingField::Pending(PendingGenesisToken {
                cell: at,
                seat,
                source_instance_id: site.card.instance_id.clone(),
            });
            game.position.phase = Phase::Genesis;
            game.position.decision_seat = seat;
            let before = game.authoritative_state();
            assert!(matches!(
                game.apply_resolve_genesis_token(
                    seat,
                    GenesisTokenChoice::PayOneMana,
                    &mut OutcomeLog::Record(&mut events)
                ),
                Err(GameError::UnsupportedMechanic(_))
            ));
            assert_eq!(before, game.authoritative_state());
            assert!(events.is_empty());
            game.apply_resolve_genesis_token(
                seat,
                GenesisTokenChoice::Decline,
                &mut OutcomeLog::Ignore,
            )
            .unwrap();
        }
    }

    #[test]
    fn incoming_override_fate_and_neighboring_footprint_bar() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            let at = cell(seat, "B2");
            let mut bonus = minion(&game, "syn:bonus", seat, "fate-bonus", false);
            bonus.location = at;
            game.position.units.push(bonus);
            let aura = super::super::super::AuraPosition {
                card: card(&game, "syn:fate-contrast", seat, "fate"),
                cells: vec![at],
                controller: seat,
                turn_counters: 0,
                visited_cells: std::collections::BTreeSet::new(),
            };
            game.position.auras.push(aura);
            for (id, unsupported) in [("syn:paid-bar", false), ("syn:paid-bar-ordinary", true)] {
                let CardFacts::Site(site) =
                    &game.rules.cards[usize::from(card_id(&game, id).0)].facts
                else {
                    panic!()
                };
                let result = game.token_may_enter_played_site(seat, "syn:power2", at, site);
                assert_eq!(
                    matches!(result, Err(GameError::UnsupportedMechanic(_))),
                    unsupported
                );
                if !unsupported {
                    assert!(result.unwrap());
                }
            }
            // Incoming clear facts override an existing unsuppressed live bar.
            game.position.auras.clear();
            put_site(&mut game, seat, "B2", "paid-bar", true, false);
            let CardFacts::Site(clear) =
                &game.rules.cards[usize::from(card_id(&game, "syn:paid-clear").0)].facts
            else {
                panic!()
            };
            assert!(
                game.token_may_enter_played_site(seat, "syn:power2", at, clear)
                    .unwrap()
            );
        }
    }
    #[test]
    fn square_geometry_guards_before_every_neighbor_bar() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            // Mirror a footprint as a set, then retain the engine's canonical anchor.
            let wanted = ["B2", "C2", "B3", "C3"].map(|at| cell(seat, at));
            let area = Cell::SQUARE_AREAS
                .into_iter()
                .find(|area| wanted.iter().all(|cell| area.contains(cell)))
                .unwrap();
            for at in ["B2", "C2", "B3", "C3"] {
                put_site(&mut game, seat, at, "land", true, false);
            }
            let anchor = area[0];
            let bar_cell = area[3];
            let bar_id = card_id(&game, "syn:bar-land");
            game.position.sites[bar_cell.index()]
                .as_mut()
                .unwrap()
                .card
                .card_id = bar_id;
            assert!(
                game.token_may_enter_cell(seat, "syn:square-power2", anchor)
                    .unwrap()
            );
            let mut bonus = minion(&game, "syn:bonus", seat, "square-bonus", false);
            bonus.location = anchor;
            game.position.units.push(bonus);
            assert!(matches!(
                game.token_may_enter_cell(seat, "syn:square-power2", anchor),
                Err(GameError::UnsupportedMechanic(_))
            ));
            let CardFacts::Site(clear) =
                &game.rules.cards[usize::from(card_id(&game, "syn:paid-clear").0)].facts
            else {
                panic!()
            };
            assert!(matches!(
                game.token_may_enter_played_site(seat, "syn:square-power2", anchor, clear),
                Err(GameError::UnsupportedMechanic(_))
            ));
            let other = area
                .into_iter()
                .find(|c| *c != anchor && *c != bar_cell)
                .unwrap();
            game.position.sites[other.index()] = None;
            assert!(
                !game
                    .token_may_enter_cell(seat, "syn:square-power2", anchor)
                    .unwrap(),
                "impossible footprint rejects before power analysis"
            );
            assert!(
                !game
                    .token_may_enter_played_site(seat, "syn:square-power2", anchor, clear)
                    .unwrap()
            );
        }
    }

    #[test]
    fn every_ability_entry_and_mixed_legacy_identity_use_one_interpreter() {
        for seat in [Seat::North, Seat::South] {
            for entry in [
                AbilityEntry::Magic,
                AbilityEntry::Genesis,
                AbilityEntry::Entry,
                AbilityEntry::Activated,
            ] {
                let mut game = fixture(seat);
                let id = card_id(&game, "syn:single-site");
                let abilities =
                    &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(id.0)].abilities;
                let program = abilities.magic.clone();
                match entry {
                    AbilityEntry::Magic => {}
                    AbilityEntry::Genesis => abilities.genesis = program,
                    AbilityEntry::Entry => abilities.entry = program,
                    AbilityEntry::Activated => abilities.activated = program,
                }
                let mut f = frame(&game, seat, "syn:single-site");
                f.entry = entry;
                game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
                choose_all(&mut game, &mut OutcomeLog::Ignore);
                assert_eq!(game.position.units.len(), 1);
            }
            let mut game = fixture(seat);
            for _ in 0..2 {
                let mut f = frame(&game, seat, "syn:mixed-identity");
                f.source.cells = vec![cell(seat, "C2")];
                game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
                choose_all(&mut game, &mut OutcomeLog::Ignore);
            }
            assert_eq!(game.position.units.len(), 16);
            let ids: std::collections::BTreeSet<_> = game
                .position
                .units
                .iter()
                .map(|u| &u.card.instance_id)
                .collect();
            assert_eq!(
                ids.len(),
                16,
                "old/new operands, same-site members and repeated operations remain disjoint"
            );
        }
    }
    #[test]
    fn multi_site_regional_deathrites_preserve_parent_tail() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            let f = frame(&game, seat, "syn:death-parent");
            let before_atlas = game.position.players[seat_index(seat)].atlas.len();
            let before_spells = game.position.players[seat_index(seat)].spellbook.len();
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            for _ in 0..100 {
                if game.position.phase == Phase::Main {
                    break;
                }
                let actions = game.legal_actions().unwrap();
                let action = actions
                    .iter()
                    .find(|a| {
                        matches!(
                            a.descriptor,
                            ActionDescriptor::ChooseAbilityLocation {
                                location: Location {
                                    region: Region::Underground | Region::Underwater,
                                    ..
                                },
                                ..
                            }
                        )
                    })
                    .unwrap_or_else(|| actions.first().expect("regional deathrite action"));
                game.apply_action(action).unwrap();
            }
            assert_eq!(game.position.phase, Phase::Main);
            assert!(game.position.units.is_empty());
            assert_eq!(
                before_atlas - game.position.players[seat_index(seat)].atlas.len(),
                7
            );
            assert_eq!(
                before_spells - game.position.players[seat_index(seat)].spellbook.len(),
                1
            );
        }
    }
    #[test]
    fn corner_geometry_and_illegal_choice_do_not_wrap_or_mutate() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            game.position.sites.fill(None);
            game.position.rubble.fill(None);
            game.position.players[seat_index(seat)].avatar.location = cell(seat, "A1");
            for at in ["A1", "A2", "B1"] {
                put_site(&mut game, seat, at, "land", true, false);
            }
            game.position.rubble[cell(seat, "B2").index()] =
                Some(identity_hash(&json!({"corner":seat})).unwrap());
            let TokenDestination::EachSite(q) = query(true) else {
                panic!()
            };
            assert_eq!(game.site_destination_cells(q, seat).len(), 4);
            let f = frame(&game, seat, "syn:single-site");
            game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
            let source = game
                .position
                .pending_ability_choice
                .as_ref()
                .unwrap()
                .frame
                .source
                .instance_id
                .clone();
            let before = game.authoritative_state();
            let mut events = Vec::new();
            assert!(matches!(
                game.apply_ability_location_choice_action(
                    seat,
                    &source,
                    Location {
                        cell: cell(seat, "B2"),
                        region: Region::Surface
                    },
                    &mut OutcomeLog::Record(&mut events)
                ),
                Err(GameError::IllegalAction)
            ));
            assert_eq!(before, game.authoritative_state());
            assert!(events.is_empty());
        }
    }
    #[test]
    #[ignore = "requires explicitly supplied source-guarded private candidate fixtures"]
    #[expect(
        clippy::too_many_lines,
        reason = "keeps native sourceful permission, custody, region and lifetime contrasts together"
    )]
    fn private_source_guarded_native_card_boundaries() {
        use sha2::{Digest, Sha256};
        use std::fmt::Write;
        let input = std::env::var("SORCERY_SITE_TOKEN_INPUT").unwrap();
        let packet: serde_json::Value =
            serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
        let mut fixtures = None;
        for guard in packet["sourceGuards"].as_array().unwrap() {
            let path = guard["path"].as_str().unwrap();
            let bytes = std::fs::read(path).unwrap();
            let mut digest = String::new();
            for byte in Sha256::digest(&bytes) {
                write!(digest, "{byte:02x}").unwrap();
            }
            assert_eq!(digest, guard["sha256"].as_str().unwrap());
            if std::path::Path::new(path).file_name().unwrap() == "fixtures.json" {
                fixtures = Some(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap());
            }
        }
        let fixtures = fixtures.unwrap();
        let mut extra = fixtures["actualFacts"].clone();
        for name in [
            "syn:caster",
            "syn:cross-caster",
            "syn:fire-caster",
            "syn:single-frog",
            "syn:single-soldier",
            "syn:gear-spell",
        ] {
            extra[name] = fixtures["syntheticFacts"][name].clone();
        }
        let mut results = Vec::new();
        for seat in [Seat::North, Seat::South] {
            let owner = super::super::super::other_seat(seat);
            for (id, facts) in fixtures["actualFacts"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, facts)| facts["cardType"] == "magic")
            {
                for region in [
                    Region::Surface,
                    Region::Underground,
                    Region::Underwater,
                    Region::Void,
                ] {
                    let mut game = fixture_with_facts(seat, Some(&extra));
                    let mut caster =
                        minion(&game, "syn:cross-caster", seat, "private-caster", false);
                    caster.card.owner = owner;
                    caster.region = region;
                    caster.location = cell(
                        seat,
                        match region {
                            Region::Surface | Region::Underwater => "A4",
                            Region::Underground => "B2",
                            Region::Void => "D1",
                        },
                    );
                    caster.summoning_sickness = false;
                    let caster_id = caster.card.instance_id.clone();
                    game.position.units.push(caster);
                    let magic = card(&game, id, owner, "private-magic");
                    let magic_id = magic.instance_id.clone();
                    game.position.players[seat_index(seat)].hand_spellbook = vec![magic];
                    let find_cast = |game: &Game| {
                        game.legal_actions().unwrap().into_iter().find(|a|matches!(&a.descriptor,ActionDescriptor::CastMagic {card_id,caster_instance_id,..} if card_id==id && caster_instance_id==&caster_id))
                    };
                    let action = find_cast(&game)
                        .expect("sourceful non-target cast issued in each legal caster region");
                    game.position.players[seat_index(seat)].mana =
                        u16::try_from(facts["manaCost"].as_u64().unwrap() - 1).unwrap();
                    assert!(find_cast(&game).is_none());
                    game.position.players[seat_index(seat)].mana = 12;
                    let mut low_affinity = game.clone();
                    for site in low_affinity.position.sites.iter_mut().flatten() {
                        if site.controller == seat {
                            site.controller = owner;
                        }
                    }
                    assert!(find_cast(&low_affinity).is_none());
                    let mut restricted = game.clone();
                    restricted.position.units[0].card.card_id =
                        card_id(&restricted, "syn:fire-caster");
                    assert!(find_cast(&restricted).is_none());
                    let before = game.authoritative_state();
                    if facts["effectProgram"]["effects"][0]["count"] == 4 {
                        assert!(matches!(
                            game.apply_action_recorded(&action),
                            Err(GameError::UnsupportedMechanic(_))
                        ));
                        assert_eq!(before, game.authoritative_state());
                        results.push(json!({"seat":seat,"card":id,"region":region,"status":"BLOCKED_R3_AS_EXPECTED"}));
                        continue;
                    }
                    let mut events = game.apply_action_recorded(&action).unwrap().0;
                    // A started non-target Magic retains controller/owner after caster departure.
                    game.position.units.clear();
                    while game.position.phase == Phase::AbilityChoice {
                        let actions = game.legal_actions().unwrap();
                        let action = actions
                            .iter()
                            .find(|a| {
                                matches!(
                                    a.descriptor,
                                    ActionDescriptor::ChooseAbilityLocation {
                                        location: Location {
                                            region: Region::Surface,
                                            ..
                                        },
                                        ..
                                    }
                                )
                            })
                            .unwrap();
                        events.extend(game.apply_action_recorded(action).unwrap().0);
                    }
                    assert_eq!(game.position.units.len(), 7);
                    assert!(
                        game.position
                            .units
                            .iter()
                            .all(|u| u.card.owner == seat && u.controller == seat)
                    );
                    assert!(
                        !game
                            .position
                            .units
                            .iter()
                            .any(|u| u.location == cell(seat, "A4"))
                    );
                    assert_eq!(game.position.players[seat_index(seat)].mana, 6);
                    assert_eq!(
                        game.position.players[seat_index(owner)]
                            .cemetery
                            .iter()
                            .filter(|c| c.instance_id == magic_id)
                            .count(),
                        1
                    );
                    assert!(
                        game.position.sites[cell(seat, "B3").index()]
                            .as_ref()
                            .unwrap()
                            .warded
                    );
                    results.push(json!({"seat":seat,"card":id,"region":region,"status":"PASS_NATIVE_ONLY","stateHash":game.state_hash().unwrap(),"events":events}));
                }
            }
            for (id, facts) in fixtures["actualFacts"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, facts)| facts["token"] == true)
            {
                let mut game = fixture_with_facts(seat, Some(&extra));
                let location = Location {
                    cell: cell(seat, "C2"),
                    region: Region::Surface,
                };
                let token = game
                    .create_token_unit(
                        seat,
                        id,
                        &source("private-token", seat, 0, None).instance_id,
                        location,
                        0,
                        100,
                    )
                    .unwrap();
                assert!(
                    token.summoning_sickness && !token.tapped && token.occupied_cells.is_none()
                );
                let token_id = token.card.instance_id.clone();
                game.finish_token_entry(
                    seat,
                    token,
                    &source("private-token", seat, 0, None).instance_id,
                    0,
                    &mut OutcomeLog::Ignore,
                )
                .unwrap();
                assert_eq!(
                    game.position.units.len(),
                    1,
                    "including an undamaged printed-zero token"
                );
                let target = UnitTarget::Minion {
                    instance_id: token_id.clone(),
                    seat,
                };
                assert!(!game.legal_actions().unwrap().iter().any(|a| matches!(&a.descriptor,
                    ActionDescriptor::MoveAndAttack {unit_instance_id,..} if unit_instance_id==&token_id)));
                game.position.players[seat_index(seat)].hand_spellbook =
                    vec![card(&game, "syn:gear-spell", seat, "private-gear")];
                let cast_gear = game
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|a| {
                        matches!(&a.descriptor,
                    ActionDescriptor::CastArtifact {bearer:Some(bearer),..} if bearer==&target)
                    })
                    .unwrap();
                game.apply_action(&cast_gear).unwrap();
                assert_eq!(
                    game.position.artifacts.len(),
                    1,
                    "exact tokens can carry a normal artifact"
                );
                let mut rested = game.clone();
                let end = rested
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|a| matches!(a.descriptor, ActionDescriptor::EndTurn))
                    .unwrap();
                rested.apply_action(&end).unwrap();
                for _ in 0..5 {
                    if rested.position.active_seat == seat && rested.position.phase == Phase::Main {
                        break;
                    }
                    let actions = rested.legal_actions().unwrap();
                    let action = actions
                        .iter()
                        .find(|a| matches!(a.descriptor, ActionDescriptor::Draw { .. }))
                        .or_else(|| {
                            actions
                                .iter()
                                .find(|a| matches!(a.descriptor, ActionDescriptor::EndTurn))
                        })
                        .unwrap();
                    rested.apply_action(action).unwrap();
                }
                assert!(rested.legal_actions().unwrap().iter().any(|a|matches!(&a.descriptor,
                    ActionDescriptor::MoveAndAttack {unit_instance_id,..} if unit_instance_id==&token_id)));
                drop(rested);
                let damage = install(
                    &mut game,
                    vec![Effect::Damage {
                        recipients: UnitSet::Target,
                        amount: 1,
                    }],
                );
                let f = game
                    .effect_frame(
                        damage,
                        AbilityEntry::Magic,
                        source("damage", seat, 0, None),
                        Some(&target),
                        None,
                        None,
                    )
                    .unwrap();
                let mut events = Vec::new();
                game.run_effect_frame(f, &mut OutcomeLog::Record(&mut events))
                    .unwrap();
                assert!(game.position.units.is_empty());
                assert!(
                    game.position
                        .players
                        .iter()
                        .all(|p| !p.cemetery.iter().any(|c| c.instance_id == token_id))
                );
                assert!(
                    events
                        .iter()
                        .any(|(kind, payload)| kind == "minion-banished"
                            && payload["instanceId"] == json!(token_id))
                );
                results.push(json!({"seat":seat,"token":id,"printedCost":facts["manaCost"],"status":"PASS_NATIVE_LIFECYCLE_ONLY","events":events}));
            }
        }
        let output = std::path::Path::new(packet["output"].as_str().unwrap())
            .with_file_name("native-candidate-results.json");
        std::fs::write(output, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
    }
    #[test]
    fn empty_group_finishes_parent_once_without_a_choice() {
        for seat in [Seat::North, Seat::South] {
            let mut game = fixture(seat);
            game.position.sites.fill(None);
            game.position.rubble.fill(None);
            let before = game.position.players[seat_index(seat)].spellbook.len();
            let f = frame(&game, seat, "syn:parent");
            let magic = f.magic.as_ref().unwrap().instance_id.clone();
            let mut events = Vec::new();
            game.run_effect_frame(f, &mut OutcomeLog::Record(&mut events))
                .unwrap();
            assert!(game.position.pending_ability_choice.is_none());
            assert!(game.position.units.is_empty());
            assert_eq!(
                before - game.position.players[seat_index(seat)].spellbook.len(),
                1
            );
            assert_eq!(
                game.position.players[seat_index(seat)]
                    .cemetery
                    .iter()
                    .filter(|card| card.instance_id == magic)
                    .count(),
                1
            );
            assert!(!events.iter().any(|(kind, _)| kind == "minion-summoned"));
        }
    }
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one matrix keeps regional C1 controls and cohort atomicity together"
    )]
    fn regional_c1_quarantine_preserves_controls_and_whole_cohorts() {
        let extra = json!({"syn:regional-bonus":{
            "cardType":"minion","attack":1,"defense":2,"manaCost":0,
            "thresholds":{"air":0,"earth":0,"fire":0,"water":0},
            "submerge":true,"burrowing":true,"otherNearbyAlliesPowerBonus":1
        }});
        for seat in [Seat::North, Seat::South] {
            for region in [Region::Surface, Region::Underwater, Region::Underground] {
                for (same_region, bar, allied, silenced) in [
                    (true, true, true, false),
                    (false, true, true, false),
                    (true, false, true, false),
                    (true, true, false, false),
                    (true, true, true, true),
                ] {
                    let mut game = fixture_with_facts(seat, Some(&extra));
                    let at = if region == Region::Underwater {
                        "D2"
                    } else {
                        "C2"
                    };
                    let site = match (region == Region::Underwater, bar) {
                        (true, true) => "bar-water",
                        (true, false) => "water",
                        (false, true) => "bar-land",
                        (false, false) => "land",
                    };
                    put_site(&mut game, seat, at, site, true, false);
                    let controller = if allied {
                        seat
                    } else {
                        super::super::super::other_seat(seat)
                    };
                    let mut provider = minion(
                        &game,
                        "syn:regional-bonus",
                        controller,
                        "regional-provider",
                        false,
                    );
                    provider.region = if same_region {
                        region
                    } else if region == Region::Surface {
                        Region::Underwater
                    } else {
                        Region::Surface
                    };
                    provider.location = cell(
                        seat,
                        if provider.region == Region::Underwater {
                            "D2"
                        } else {
                            at
                        },
                    );
                    if silenced {
                        provider.temporary_modifiers.grant(
                            crate::ability::TemporaryModifierKind::Silence,
                            1,
                            source("silence", seat, 0, None).instance_id,
                        );
                    }
                    game.position.units.push(provider);
                    let location = Location {
                        cell: cell(seat, at),
                        region,
                    };
                    let result = game.token_may_enter_location(seat, "syn:power2", location);
                    let held = same_region && bar && allied && !silenced;
                    assert_eq!(
                        matches!(result, Err(GameError::UnsupportedMechanic(_))),
                        held
                    );
                    if !held {
                        assert!(result.unwrap());
                    }
                    let result = game.create_token_unit(
                        seat,
                        "syn:power2",
                        &source("ctor", seat, 0, None).instance_id,
                        location,
                        0,
                        100,
                    );
                    assert_eq!(
                        matches!(result, Err(GameError::UnsupportedMechanic(_))),
                        held
                    );
                    // Ordinary prospective entry keeps its existing Surface-only semantics.
                    let facts = game.token_minion_facts("syn:power2").unwrap();
                    let power =
                        game.prospective_minion_entry_power(seat, facts, &[location.cell], None);
                    let surface_bonus =
                        game.position.units[0].region == Region::Surface && allied && !silenced;
                    assert_eq!(power, if surface_bonus { 3 } else { 2 });
                }
                let mut game = fixture_with_facts(seat, Some(&extra));
                game.position.sites.fill(None);
                game.position.rubble.fill(None);
                let water = region == Region::Underwater;
                put_site(
                    &mut game,
                    seat,
                    "C2",
                    if water { "water" } else { "land" },
                    true,
                    false,
                );
                put_site(
                    &mut game,
                    seat,
                    "D2",
                    if water { "bar-water" } else { "bar-land" },
                    true,
                    false,
                );
                let mut provider =
                    minion(&game, "syn:regional-bonus", seat, "cohort-provider", false);
                provider.region = region;
                provider.location = cell(seat, "D2");
                game.position.units.push(provider);
                install(
                    &mut game,
                    vec![Effect::SummonToken {
                        token: "syn:power2".into(),
                        count: 1,
                        destination: query(true),
                    }],
                );
                let f = frame(&game, seat, "north-spell-49");
                game.run_effect_frame(f, &mut OutcomeLog::Ignore).unwrap();
                let mut commits = 0;
                loop {
                    let actions = game.legal_actions().unwrap();
                    let action=actions.iter().find(|a|matches!(a.descriptor,ActionDescriptor::ChooseAbilityLocation {location,..} if location.region==region)).unwrap();
                    let before = game.authoritative_state();
                    match game.apply_action_recorded(action) {
                        Ok((events, _)) => {
                            commits += 1;
                            assert!(!events.iter().any(|(kind, _)| kind == "minion-summoned"));
                        }
                        Err(GameError::UnsupportedMechanic(reason)) => {
                            assert_eq!(
                                reason,
                                "token entry power timing with ongoing bonuses is unresolved"
                            );
                            assert_eq!(before, game.authoritative_state());
                            break;
                        }
                        other => panic!("unexpected regional cohort result {other:?}"),
                    }
                    assert_eq!(game.position.units.len(), 1);
                }
                assert_eq!(commits, 1);
                assert_eq!(game.position.units.len(), 1);
            }
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the approved proof matrix keeps each simultaneous owner and its continuation assertions together"
)]
fn compiled_damage_wakes_a_power_provider_before_deaths_and_resumes_its_frame_once() {
    for survives in [false, true] {
        let mut game = fixture_game();
        let provider_card = card_id(&game, "north-spell-48");
        let rules = Arc::get_mut(&mut game.rules).unwrap();
        let crate::facts::CardFacts::Minion(facts) =
            &mut rules.cards[usize::from(provider_card.0)].facts
        else {
            panic!("provider")
        };
        facts.defense = if survives { 8 } else { 2 };
        facts.other_nearby_allies_power_bonus = true;
        for id in ["south-spell-1", "south-spell-2"] {
            let card = rules.cards.iter_mut().find(|c| c.id == id).unwrap();
            let crate::facts::CardFacts::Minion(facts) = &mut card.facts else {
                panic!("recipient")
            };
            facts.defense = 3;
            facts.damage_prevention = None;
        }
        let mut provider = minion(
            &game,
            "north-spell-48",
            Seat::North,
            "sleeping-provider",
            false,
        );
        provider.disabled_until_damaged = true;
        game.position.units = vec![
            minion(
                &game,
                "south-spell-1",
                Seat::North,
                "first-recipient",
                false,
            ),
            minion(
                &game,
                "south-spell-2",
                Seat::North,
                "second-recipient",
                false,
            ),
            provider,
        ];
        let recipients = UnitSet::Query(UnitCohort {
            area: UnitArea::Source,
            kind: Some(super::super::UnitKind::Minion),
            controller: ControllerRelation::Any,
            relation: None,
            exclude_source: false,
        });
        let magic = install(
            &mut game,
            vec![
                Effect::Damage {
                    recipients,
                    amount: 3,
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
                source("wake-group", Seat::North, 0, None),
                None,
                None,
                None,
            )
            .unwrap();
        let before = game.position.players[0].hand_spellbook.len();
        let mut silent = game.clone();
        let mut events = Vec::new();
        game.run_effect_frame(frame.clone(), &mut OutcomeLog::Record(&mut events))
            .unwrap();
        silent
            .run_effect_frame(frame, &mut OutcomeLog::Ignore)
            .unwrap();
        assert_eq!(game.position, silent.position);
        if survives {
            assert!(game.position.pending_deathrites.is_none());
            assert_eq!(game.position.units.len(), 3);
            assert_eq!(game.position.players[0].hand_spellbook.len(), before + 1);
        } else {
            assert!(game.position.pending_deathrites.is_some());
            assert_eq!(
                game.position.players[0].hand_spellbook.len(),
                before,
                "program tail must wait for Deathrites"
            );
            let action = game
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| matches!(a.descriptor, ActionDescriptor::OrderTriggers { .. }))
                .unwrap();
            let owned = game.clone().apply_action_owned(&action).unwrap();
            let (resolved, _) = game.apply_action_recorded(&action).unwrap();
            silent.apply_action(&action).unwrap();
            assert_eq!(game.position, silent.position);
            assert_eq!(game.position, owned.position);
            assert_eq!(
                game.legal_actions().unwrap(),
                owned.legal_actions().unwrap()
            );
            assert_eq!(
                resolved
                    .iter()
                    .filter(|(kind, _)| kind == "spell-drawn")
                    .count(),
                3
            );
            assert!(game.position.pending_deathrites.is_none());
            assert!(game.position.units.is_empty());
            assert_eq!(game.position.players[0].hand_spellbook.len(), before + 3);
        }
    }
}
