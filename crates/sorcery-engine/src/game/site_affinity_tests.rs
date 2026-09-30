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

#[test]
fn rubble_land_classification_preserves_water_and_void_controls() {
    let cell = Cell::parse("C4").unwrap();
    for (elements, affinity, water) in [
        (vec![], counts(0, 0, 0, 0), false),
        (vec!["fire", "air"], counts(0, 2, 0, 2), false),
        (vec!["water"], counts(0, 0, 2, 0), true),
        (vec!["earth", "water"], counts(2, 0, 1, 0), true),
    ] {
        let game = fixture(&elements, Some(affinity));
        assert_eq!(game.is_water_site(cell), water);
        assert_eq!(game.is_land_site(cell), !water);
    }

    let mut rubble = fixture(&["earth"], None);
    rubble.position.sites[cell.index()] = None;
    rubble.position.rubble[cell.index()] =
        Some(identity_hash(&json!({"fixture":"uncovered-rubble","cell":cell})).unwrap());
    assert!(rubble.surface_location_exists(cell));
    assert!(!rubble.is_water_site(cell));
    assert!(rubble.is_land_site(cell));
    assert_eq!(rubble.elemental_affinities(Seat::North), [0; 4]);
    assert_eq!(rubble.elemental_affinities(Seat::South), [1, 0, 0, 0]);
    rubble.position.sites[Cell::parse("C1").unwrap().index()] = None;
    assert_eq!(rubble.elemental_affinities(Seat::South), [0; 4]);
    assert!(!rubble.surface_location_exists(Cell::parse("A1").unwrap()));
    assert!(!rubble.is_water_site(Cell::parse("A1").unwrap()));
    assert!(!rubble.is_land_site(Cell::parse("A1").unwrap()));

    overlay(&mut rubble, "flood");
    assert!(rubble.is_water_site(cell));
    assert!(!rubble.is_land_site(cell));
    assert_eq!(rubble.effective_site_affinity(cell, [0; 4]), [0, 0, 1, 0]);
    assert_eq!(rubble.elemental_affinities(Seat::North), [0; 4]);
    overlay(&mut rubble, "drought");
    assert!(!rubble.is_water_site(cell));
    assert!(rubble.is_land_site(cell));
    assert_eq!(rubble.effective_site_affinity(cell, [0; 4]), [0; 4]);

    let mut flood_wins = fixture(&["earth"], None);
    flood_wins.position.sites[cell.index()] = None;
    flood_wins.position.rubble[cell.index()] =
        Some(identity_hash(&json!({"fixture":"flood-wins-rubble","cell":cell})).unwrap());
    overlay(&mut flood_wins, "drought");
    overlay(&mut flood_wins, "flood");
    assert!(flood_wins.is_water_site(cell), "later Flood wins Drought");

    let mut fate_only = fixture(&["earth"], None);
    fate_only.position.sites[cell.index()] = None;
    fate_only.position.rubble[cell.index()] =
        Some(identity_hash(&json!({"fixture":"fate-rubble","cell":cell})).unwrap());
    overlay(&mut fate_only, "fate");
    assert!(
        fate_only.is_land_site(cell),
        "Fate excludes Ordinary Rubble"
    );

    let mut protected = fixture(&["earth"], Some(counts(2, 0, 0, 0)));
    let site_id = protected.position.sites[cell.index()]
        .as_ref()
        .unwrap()
        .card
        .card_id;
    let CardFacts::Site(facts) =
        &mut Arc::get_mut(&mut protected.rules).unwrap().cards[usize::from(site_id.0)].facts
    else {
        panic!("site")
    };
    facts.cannot_be_moved_destroyed_or_modified = true;
    overlay(&mut protected, "flood");
    assert!(!protected.is_water_site(cell));
    assert!(protected.is_land_site(cell));
    assert_eq!(
        protected.effective_site_affinity(cell, [2, 0, 0, 0]),
        [2, 0, 0, 0]
    );
}
