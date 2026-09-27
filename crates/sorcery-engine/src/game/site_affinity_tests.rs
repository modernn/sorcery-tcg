//! Site classification and counted production share overlays without losing multiplicity.
use super::*;
use crate::synthetic::selfplay_manifest_with;

fn fixture(elements: &[&str], affinity: Option<Value>) -> Game {
    let encoded = selfplay_manifest_with(401, |m| {
        let zero = json!({"earth":0,"fire":0,"water":0,"air":0});
        let mut site = json!({"cardType":"site","elements":elements});
        if let Some(affinity) = affinity {
            site["siteAffinity"] = affinity;
        }
        m["cards"] = json!({
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":site,
            "provider":{"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":zero,"siteProvidesNoThreshold":true},
            "flood":{"cardType":"aura","manaCost":0,"thresholds":zero,"affectedSitesAreFlooded":true},
            "drought":{"cardType":"aura","manaCost":0,"thresholds":zero,"affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold":true},
            "fate":{"cardType":"aura","manaCost":0,"thresholds":zero,"affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities":true}
        });
        m["decks"] = json!({
            "north":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":vec!["provider";3]},
            "south":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":["flood","drought","fate"]}
        });
    });
    let mut game = Game::from_manifest_json(&encoded).unwrap();
    for (seat, cell) in [(Seat::North, "C4"), (Seat::South, "C1")] {
        let card = game.position.players[seat_index(seat)].hand_atlas.remove(0);
        game.position.sites[Cell::parse(cell).unwrap().index()] = Some(SitePosition {
            card,
            controller: seat,
            last_flight_turn: None,
            warded: false,
        });
    }
    game
}
fn overlay(game: &mut Game, name: &str) {
    let index = game.position.players[1]
        .hand_spellbook
        .iter()
        .position(|card| game.rules.cards[usize::from(card.card_id.0)].id == name)
        .unwrap();
    let card = game.position.players[1].hand_spellbook.remove(index);
    game.position.auras.push(AuraPosition {
        card,
        cells: vec![Cell::parse("C4").unwrap()],
        controller: Seat::South,
        turn_counters: 0,
        visited_cells: BTreeSet::new(),
    });
}
fn counts(earth: u8, fire: u8, water: u8, air: u8) -> Value {
    json!({"earth":earth,"fire":fire,"water":water,"air":air})
}

#[test]
fn each_printed_double_affinity_satisfies_two_thresholds_without_changing_site_classification() {
    for (index, name) in ["earth", "fire", "water", "air"].into_iter().enumerate() {
        let mut production = counts(0, 0, 0, 0);
        production[name] = json!(2);
        let game = fixture(&[name], Some(production.clone()));
        let legacy = fixture(&[name], None);
        let required = json!({"cardType":"minion","attack":1,"defense":1,"manaCost":0,"thresholds":production});
        let CardFacts::Minion(facts) =
            crate::facts::parse_card_definition("required", &required).unwrap()
        else {
            panic!("minion")
        };
        for seat in [Seat::North, Seat::South] {
            assert_eq!(game.elemental_affinities(seat)[index], 2);
            assert_eq!(legacy.elemental_affinities(seat)[index], 1);
            assert!(game.thresholds_met(seat, facts.thresholds));
            assert!(!legacy.thresholds_met(seat, facts.thresholds));
        }
        let cell = Cell::parse("C4").unwrap();
        assert_eq!(game.is_water_site(cell), name == "water");
        assert_eq!(game.is_land_site(cell), name != "water");
    }
}

#[test]
fn flooded_and_water_only_overlays_preserve_existing_water_multiplicity() {
    let mut water = fixture(&["water"], Some(counts(0, 0, 2, 0)));
    overlay(&mut water, "flood");
    assert_eq!(water.elemental_affinities(Seat::North), [0, 0, 2, 0]);
    overlay(&mut water, "fate");
    assert_eq!(water.elemental_affinities(Seat::North), [0, 0, 2, 0]);
    let mut land = fixture(&["earth"], Some(counts(2, 0, 0, 0)));
    overlay(&mut land, "flood");
    assert_eq!(land.elemental_affinities(Seat::North), [2, 0, 1, 0]);
    overlay(&mut land, "fate");
    assert_eq!(land.elemental_affinities(Seat::North), [0, 0, 1, 0]);
    assert!(land.is_water_site(Cell::parse("C4").unwrap()));
}

#[test]
fn drought_removes_only_water_and_later_overlays_restore_printed_counts() {
    let mut game = fixture(&["earth", "water"], Some(counts(2, 0, 2, 0)));
    overlay(&mut game, "drought");
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 0, 0]);
    assert!(game.is_land_site(Cell::parse("C4").unwrap()));
    overlay(&mut game, "flood");
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 2, 0]);
    game.position.auras.clear();
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 2, 0]);
}

#[test]
fn site_suppression_and_minion_production_both_obey_ability_loss() {
    let mut game = fixture(&["earth"], Some(counts(2, 0, 0, 0)));
    let card = game.position.players[0].hand_spellbook.remove(0);
    let id = card.instance_id.clone();
    let card_id = card.card_id;
    let CardFacts::Minion(facts) =
        &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
    else {
        panic!("minion")
    };
    facts.provides = Some(Element::Air);
    game.position.units.push(
        SummonPlacement {
            card,
            controller: Seat::North,
            lance_count: 0,
            location: Cell::parse("C4").unwrap(),
            occupied_cells: None,
            region: Region::Surface,
            stealthed: false,
            warded: false,
        }
        .into_unit(),
    );
    assert_eq!(game.elemental_affinities(Seat::North), [0, 0, 0, 1]);
    game.position.units[0]
        .temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, id);
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 0, 0]);
    game.position.units[0]
        .temporary_modifiers
        .take(TemporaryModifierKind::Silence);
    game.position.units[0].disabled_until_damaged = true;
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 0, 0]);
    game.position.units.clear();
    assert_eq!(game.elemental_affinities(Seat::North), [2, 0, 0, 0]);
}

#[test]
fn site_affinity_counts_require_canonical_keys_bounds_and_element_membership() {
    for value in [
        json!(null),
        json!([]),
        json!({"earth":2}),
        counts(0, 0, 0, 0),
        counts(2, 1, 0, 0),
        json!({"earth":2.5,"fire":0,"water":0,"air":0}),
        counts(101, 0, 0, 0),
        json!({"earth":2,"fire":0,"water":0,"air":0,"extra":0}),
    ] {
        let card = json!({"cardType":"site","elements":["earth"],"siteAffinity":value});
        assert!(crate::facts::parse_card_definition("invalid", &card).is_err());
    }
}

#[test]
fn unmodifiable_sites_keep_printed_affinity_classification_and_abilities_under_auras() {
    for aura in ["flood", "drought", "fate"] {
        for water in [false, true] {
            let elements = if water {
                vec!["earth", "water"]
            } else {
                vec!["earth"]
            };
            let mut game = fixture(&elements, Some(counts(2, 0, if water { 2 } else { 0 }, 0)));
            let cell = Cell::parse("C4").unwrap();
            let card_id = game.position.sites[cell.index()]
                .as_ref()
                .unwrap()
                .card
                .card_id;
            let CardFacts::Site(facts) =
                &mut Arc::get_mut(&mut game.rules).unwrap().cards[usize::from(card_id.0)].facts
            else {
                panic!("site")
            };
            facts.cannot_be_moved_destroyed_or_modified = true;
            overlay(&mut game, aura);
            assert_eq!(
                game.elemental_affinities(Seat::North),
                [2, 0, if water { 2 } else { 0 }, 0]
            );
            assert_eq!(game.is_water_site(cell), water);
            assert!(!game.site_abilities_lost(cell));
            let card = game.position.players[0].hand_spellbook.remove(0);
            game.position.units.push(
                SummonPlacement {
                    card,
                    controller: Seat::North,
                    lance_count: 0,
                    location: cell,
                    occupied_cells: None,
                    region: Region::Surface,
                    stealthed: false,
                    warded: false,
                }
                .into_unit(),
            );
            assert_eq!(
                game.elemental_affinities(Seat::North),
                [2, 0, if water { 2 } else { 0 }, 0],
                "protected affinity cannot be suppressed even under Fate"
            );
        }
    }
}
